//! What a loop needs to leave the closure engine and come back (ADR-0020 round 3): the registry a
//! compilation pass fills for every statement inside a loop, and the per-loop `OsrSite`.
//!
//! A hot `while` loop runs as machine code (`jit/osr.rs`). Machine code only takes the happy path:
//! on anything else (an overflow, a bounds failure, a type it did not expect, `break`, `return`)
//! it writes its variables back to their slots and names the statement it stopped at. The closure
//! engine then runs that statement and the rest of the iteration with the statement codes recorded
//! here, which are the very closures the loop body is made of, so the walker's semantics, panics
//! and locations are produced by the code that always produces them.

use super::Code;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::R;
use crate::interpreter::int::IntTy;
use crate::interpreter::value::Value;
use crate::parser::Statement;
use crate::source_map::Location;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

/// A statement's compiled code, shared by its block and the registry.
pub(crate) type SCode = Rc<dyn Fn(&mut Interpreter, usize) -> R>;
type S = &'static Statement<'static>;

pub(crate) fn share(c: Code) -> SCode {
    Rc::from(c)
}

/// How a slot holds its value (see `compile.rs`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum SlotKind {
    Plain,
    /// Always an integer of this width.
    Typed(IntTy),
    /// A walker slot (`island::boxed`).
    Boxed,
}

/// What `scoped` does when a block ends: poison the guards of `poison` on a panic, clear `clear`.
pub(crate) struct BlockInfo {
    pub(crate) poison: Vec<usize>,
    pub(crate) clear: Vec<usize>,
}

/// A `for` loop's variable slot and the scope that holds it (released after every iteration).
pub(crate) struct ForInfo {
    pub(crate) slot: Option<usize>,
    pub(crate) scope: BlockInfo,
}

/// Statement codes, block scopes and `let` slots of the statements inside loops of one compiled
/// function (or closure body), keyed by AST address.
#[derive(Default)]
pub(crate) struct Reg {
    stmts: RefCell<HashMap<usize, SCode>>,
    blocks: RefCell<HashMap<usize, Rc<BlockInfo>>>,
    lets: RefCell<HashMap<usize, usize>>,
    fors: RefCell<HashMap<usize, Rc<ForInfo>>>,
    kinds: RefCell<Vec<SlotKind>>,
}

fn key<T: ?Sized>(p: &T) -> usize {
    p as *const T as *const u8 as usize
}

impl Reg {
    pub(crate) fn record_stmt(&self, s: S, c: &SCode) {
        self.stmts.borrow_mut().insert(key(s), c.clone());
    }
    pub(crate) fn record_block(&self, list: &'static [S], info: BlockInfo) {
        self.blocks.borrow_mut().insert(list.as_ptr() as usize, Rc::new(info));
    }
    pub(crate) fn record_for(&self, s: S, slot: Option<usize>, scope: BlockInfo) {
        self.fors.borrow_mut().insert(key(s), Rc::new(ForInfo { slot, scope }));
    }
    pub(crate) fn for_info(&self, s: S) -> Option<Rc<ForInfo>> {
        self.fors.borrow().get(&key(s)).cloned()
    }
    pub(crate) fn record_let(&self, s: S, slot: usize) {
        self.lets.borrow_mut().insert(key(s), slot);
    }
    /// The final slot kinds of the pass (set once the pass is done).
    pub(crate) fn finish(&self, slot_ty: &[Option<IntTy>], boxed: &HashSet<usize>) {
        *self.kinds.borrow_mut() = slot_ty
            .iter()
            .enumerate()
            .map(|(i, t)| match t {
                _ if boxed.contains(&i) => SlotKind::Boxed,
                Some(t) => SlotKind::Typed(*t),
                None => SlotKind::Plain,
            })
            .collect();
    }
    pub(crate) fn code(&self, s: S) -> Option<SCode> {
        self.stmts.borrow().get(&key(s)).cloned()
    }
    pub(crate) fn block(&self, list: &'static [S]) -> Option<Rc<BlockInfo>> {
        self.blocks.borrow().get(&(list.as_ptr() as usize)).cloned()
    }
    pub(crate) fn let_slot(&self, s: S) -> Option<usize> {
        self.lets.borrow().get(&key(s)).copied()
    }
    pub(crate) fn kind(&self, slot: usize) -> Option<SlotKind> {
        self.kinds.borrow().get(slot).copied()
    }
}

/// One `while` loop of compiled code: the locals visible at it, its hotness and its machine code.
pub(crate) struct OsrSite {
    pub(crate) stmt: S,
    /// Locals visible at the loop, in declaration order (a later entry shadows an earlier one).
    pub(crate) names: Vec<(&'static str, usize)>,
    /// The function's registry (owned by `Interpreter::osr_regs`; the codes in it own this site).
    pub(crate) reg: Weak<Reg>,
    hits: Cell<u32>,
    /// Never worth trying again (the loop is outside the machine-code subset).
    pub(crate) dead: Cell<bool>,
    #[cfg(feature = "jit")]
    pub(crate) units: RefCell<super::jit::osr::Units>,
}

impl OsrSite {
    pub(crate) fn new(stmt: S, names: Vec<(&'static str, usize)>, reg: Weak<Reg>) -> OsrSite {
        OsrSite {
            stmt,
            names,
            reg,
            hits: Cell::new(0),
            dead: Cell::new(false),
            #[cfg(feature = "jit")]
            units: RefCell::new(Default::default()),
        }
    }

    /// One more iteration ran in the closure engine.
    #[inline(always)]
    pub(crate) fn tick(&self) {
        self.hits.set(self.hits.get().saturating_add(1));
    }

    /// Wait another `jit_loop_hot` iterations before trying again.
    pub(crate) fn cool(&self) {
        self.hits.set(0);
    }

    /// Should the loop head try machine code now?
    #[inline(always)]
    pub(crate) fn armed(&self, it: &Interpreter) -> bool {
        !self.dead.get() && self.hits.get() >= it.jit_loop_hot && it.jit_enabled && it.hot.is_none()
    }

    /// The slot a name in the loop refers to, if it is a local visible at the loop.
    pub(crate) fn slot_of(&self, name: &str) -> Option<usize> {
        self.names.iter().rev().find(|(n, _)| *n == name).map(|(_, s)| *s)
    }
}

/// A loop head that never enters machine code (`--no-default-features`): run the closures.
#[cfg(not(feature = "jit"))]
pub(crate) fn enter(_: &mut Interpreter, _: usize, _: &OsrSite) -> R<bool> {
    Ok(false)
}

#[cfg(not(feature = "jit"))]
pub(crate) fn enter_for(_: &mut Interpreter, _: usize, _: &OsrSite, _: i128, _: i128, _: IntTy) -> R<Option<i128>> {
    Ok(None)
}

#[cfg(feature = "jit")]
pub(crate) use super::jit::osr::{enter, enter_for};

/// Release the locals of a finished block as `scoped` does (`compile.rs`).
pub(crate) fn close_block<T>(it: &mut Interpreter, b: usize, info: Option<&BlockInfo>, r: R<T>) -> R<T> {
    let Some(info) = info else { return r };
    if info.clear.is_empty() {
        return r;
    }
    if matches!(&r, Err(crate::interpreter::error::Ctl::Err(e)) if e.kind == crate::interpreter::error::ErrorKind::Panic) {
        for s in &info.poison {
            if let Value::Ref(p) = &it.rstack[b + s] {
                crate::interpreter::scope::poison_if_guard(&p.slot.borrow());
            }
        }
    }
    for s in &info.clear {
        super::fast::set(it, b + s, Value::Unit);
    }
    r
}

/// The location the closure engine marks before running statement `s`.
pub(crate) fn loc(s: S) -> Option<&'static Location> {
    crate::interpreter::eval_expr::stmt_loc(s)
}
