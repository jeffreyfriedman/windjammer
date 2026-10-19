//! AST -> closure tree for `--engine=resolved`: the driver, slot typing and statements
//! (expressions are in `expr.rs`).
//!
//! Subset: parameters bound by name (no patterns, no `self`, no generics, not async/extern);
//! statements `let x [: T] = e` / `let _ = e`, assignment and compound assignment to a local,
//! `return`, `if`/`else`, `while`, `loop`, `for x in <iterable>`, `break [value]`, `continue`
//! (labels included), expression statements. Anything else returns `None` and the function runs
//! in the walker.
//!
//! Slot typing: a local whose every write provably stores an integer of one `small` width `T` is a
//! typed slot; reads of it compile to raw `i64` code (`typed.rs`). The proof is by construction:
//! each write site checks the static width of what it stores (`assign_ok`); a write that could
//! change the width (a literal that might not fit, another width, a non-integer) poisons the slot
//! and the function is compiled again with that slot untyped.

use super::expr::{self, X};
use super::island::{self, Island};
use super::fast::Cond;
use super::typed::{self, small, ICode, I};
use super::{Body, Code, CompiledFn};
use crate::interpreter::engine::{FuncDef, Interpreter};
use crate::interpreter::error::{Ctl, R};
use crate::interpreter::eval_expr::stmt_loc;
use crate::interpreter::eval_stmt::loop_exit;
use crate::interpreter::int::IntTy;
use crate::interpreter::value::*;
use crate::parser::{BinaryOp, CompoundOp, Expression, Pattern, Statement, Type};
use crate::source_map::Location;
use std::collections::HashSet;
use std::rc::Rc;

pub(super) type E = &'static Expression<'static>;
type S = &'static Statement<'static>;
type Loc = Option<&'static Location>;

pub(super) struct Cx<'a> {
    pub(super) it: &'a mut Interpreter,
    pub(super) module: usize,
    scopes: Vec<Vec<(&'static str, usize)>>,
    nslots: usize,
    /// Static width of each slot (`None`: any value).
    slot_ty: Vec<Option<IntTy>>,
    poison: &'a HashSet<usize>,
    conflicts: Vec<usize>,
    /// A `return` whose value is not a typed integer was compiled (no typed body).
    untyped_return: bool,
    /// Boxed locals (named by a walker island, see `island.rs`), and those found to need it.
    boxed: &'a HashSet<usize>,
    need: Vec<usize>,
    /// The function's declared return type (the walker's `scope.ret_ty` inside islands).
    pub(super) ret: Option<&'static Type>,
    /// The impl's type (the walker's `scope.self_ty` inside islands).
    pub(super) self_ty: Option<Rc<str>>,
    /// The walker hands this function's tail an expected type at entry (see `needs_hint`).
    tail_hint: bool,
}

impl Cx<'_> {
    pub(super) fn local(&self, name: &str) -> Option<usize> {
        self.scopes.iter().rev().flat_map(|s| s.iter().rev()).find(|(n, _)| *n == name).map(|(_, i)| *i)
    }
    pub(super) fn slot_ty(&self, slot: usize) -> Option<IntTy> {
        self.slot_ty[slot]
    }
    fn declare(&mut self, name: &'static str, ty: Option<IntTy>) -> usize {
        let i = self.nslots;
        self.nslots += 1;
        self.slot_ty.push(ty.filter(|t| small(*t) && !self.poison.contains(&i) && !self.boxed.contains(&i)));
        self.scopes.last_mut().expect("scope").push((name, i));
        i
    }
    /// Open / close a name scope (a `match` arm).
    pub(super) fn enter(&mut self) {
        self.scopes.push(Vec::new());
    }
    pub(super) fn leave(&mut self) {
        self.scopes.pop();
    }
    pub(super) fn declare_boxed(&mut self, name: &'static str) -> usize {
        let i = self.declare(name, None);
        self.need_box(i);
        i
    }
    /// Declare a local (`ty`: a static integer width, if any).
    pub(super) fn declare_local(&mut self, name: &'static str) -> usize {
        self.declare(name, None)
    }
    pub(super) fn nslots(&self) -> usize {
        self.nslots
    }
    /// Boxed slots of the innermost name scope (to poison on a panic).
    pub(super) fn boxed_here(&self) -> Vec<usize> {
        self.boxed_in_scope()
    }
    pub(super) fn is_boxed(&self, slot: usize) -> bool {
        self.boxed.contains(&slot)
    }
    /// `slot` must be boxed (the function is compiled again if it is not yet).
    pub(super) fn need_box(&mut self, slot: usize) {
        if !self.boxed.contains(&slot) {
            self.need.push(slot);
        }
    }
    /// Is `e` a bare read of a boxed local (which may hold a reference)?
    pub(super) fn boxed_ident(&self, e: E) -> bool {
        matches!(e, Expression::Identifier { name, .. } if self.local(name).is_some_and(|s| self.is_boxed(s)))
    }
    /// Every local declared so far and still in scope (shadowed ones too), in declaration order.
    pub(super) fn all_locals(&self) -> Vec<(&'static str, usize)> {
        self.scopes.iter().flatten().copied().collect()
    }
    /// The visible locals among `names`, in declaration order.
    pub(super) fn visible_locals(&self, names: &HashSet<&str>) -> Vec<(&'static str, usize)> {
        let mut v: Vec<(&'static str, usize)> = Vec::new();
        for n in names {
            if let Some(s) = self.local(n) {
                let name = self.scopes.iter().flatten().find(|(_, i)| *i == s).map(|(m, _)| *m).expect("declared");
                v.push((name, s));
            }
        }
        v.sort_by_key(|(_, s)| *s);
        v
    }
    /// Boxed slots declared in the innermost name scope.
    fn boxed_in_scope(&self) -> Vec<usize> {
        self.scopes.last().map(|s| s.iter().map(|(_, i)| *i).filter(|i| self.is_boxed(*i)).collect()).unwrap_or_default()
    }
    /// Does storing a value of static width `rhs` (`None`: not a typed integer; with its value
    /// when constant) into typed `slot` keep the slot's width, for `=` (`op` `None`) or `op=`?
    /// Records a conflict (the slot is untyped on the next compile) when it does not.
    fn assign_ok(&mut self, slot: usize, rhs: Option<(IntTy, Option<i64>)>, op: Option<BinaryOp>) -> bool {
        let Some(t) = self.slot_ty[slot] else { return false };
        let full = t.min() <= i64::MIN as i128 && t.max() >= i64::MAX as i128;
        let ok = match (rhs, op) {
            (None, _) => false,
            // `retag`: a literal adopts the slot's width when it fits
            (Some((u, k)), None) => u == t || (u == IntTy::Lit && (full || k.is_some_and(|k| t.fits(k as i128)))),
            (Some((u, _)), Some(op)) => typed::result_ty(op, t, u) == Some(t),
        };
        if !ok {
            self.conflicts.push(slot);
        }
        ok
    }
}

pub(super) fn compile_fn(it: &mut Interpreter, f: &Rc<FuncDef>) -> Option<CompiledFn> {
    if f.is_extern || f.is_async || !f.type_params.is_empty() {
        return None;
    }
    let has_self = has_self(f);
    if f.params.iter().enumerate().any(|(i, p)| (p.name == "self" && !(i == 0 && has_self)) || !matches!(p.pattern, None | Some(Pattern::Identifier(_)))) {
        return None;
    }
    // a parameter the body mutates holds a `&mut` place (inferred `&mut`): boxed. `self` (slot 0 of
    // a method) is always boxed: it may be the receiver's place.
    let mutated = it.param_mutation(f);
    let shift = has_self as usize;
    let mut boxed: HashSet<usize> = mutated.iter().enumerate().filter(|(_, m)| **m).map(|(i, _)| i + shift).collect();
    if has_self {
        boxed.insert(0);
    }
    let unit = Unit { module: f.module, ret: f.ret, self_ty: f.self_ty.clone(), tail_hint: tail_hint(f) };
    fixpoint(it, &unit, boxed, |cx| compile_with(cx, f))
}

/// What a compilation unit (a function or a closure body) hands its `Cx`.
pub(super) struct Unit {
    pub(super) module: usize,
    pub(super) ret: Option<&'static Type>,
    pub(super) self_ty: Option<Rc<str>>,
    pub(super) tail_hint: bool,
}

/// Compile with `body` until slot typing and boxing are stable: a conflicting write poisons a typed
/// slot, an island naming a local boxes it, and the unit is compiled again.
pub(super) fn fixpoint<T>(it: &mut Interpreter, u: &Unit, mut boxed: HashSet<usize>, mut body: impl FnMut(&mut Cx) -> Option<T>) -> Option<T> {
    let mut poison = HashSet::new();
    loop {
        let mut cx = Cx {
            it: &mut *it,
            module: u.module,
            scopes: vec![Vec::new()],
            nslots: 0,
            slot_ty: Vec::new(),
            poison: &poison,
            conflicts: Vec::new(),
            untyped_return: false,
            boxed: &boxed,
            need: Vec::new(),
            ret: u.ret,
            self_ty: u.self_ty.clone(),
            tail_hint: u.tail_hint,
        };
        let r = body(&mut cx);
        let conflicts = std::mem::take(&mut cx.conflicts);
        let need = std::mem::take(&mut cx.need);
        if (conflicts.is_empty() && need.is_empty()) || r.is_none() {
            return r;
        }
        poison.extend(conflicts);
        boxed.extend(need);
    }
}

fn compile_with(cx: &mut Cx, f: &Rc<FuncDef>) -> Option<CompiledFn> {
    let param_int: Vec<Option<IntTy>> =
        f.params.iter().map(|p| IntTy::from_type(&p.type_).filter(|t| *t != IntTy::Lit)).collect();
    let params: Vec<usize> = f.params.iter().zip(&param_int).map(|(p, t)| cx.declare(p.name.as_str(), *t)).collect();
    let param_typed = params.iter().map(|s| cx.slot_ty[*s].is_some()).collect();
    let ret_int = f.ret.and_then(IntTy::from_type).filter(|t| *t != IntTy::Lit);
    let typed_ret = ret_int.filter(|t| small(*t));
    let (mut init, tail) = stmt_list(cx, f.body, typed_ret.is_some())?;
    // boxed parameters move into walker slots before the body runs
    let boxed_params: Vec<usize> = params.iter().copied().filter(|s| cx.is_boxed(*s)).collect();
    if !boxed_params.is_empty() {
        let bp = boxed_params.clone();
        let prologue: Code = Box::new(move |it, b| {
            for s in &bp {
                let v = std::mem::replace(&mut it.rstack[b + s], Value::Unit);
                it.rstack[b + s] = island::boxed(v);
            }
            Ok(Value::Unit)
        });
        init.insert(0, Step::Code(None, prologue));
    }
    // a panic leaving the function poisons the lock guards its variables hold (the walker's `exit_scope_vars`)
    let top = cx.boxed_in_scope();
    let body = match (typed_ret, tail) {
        (Some(t), Some((loc, Tail::Int(i, u)))) if !cx.untyped_return => Body::I(poisoning_i(typed_seq(init, u, loc, i.into_code()), top), u, t),
        (_, tail) => Body::V(poisoning(seq(init, tail.map(|(l, t)| (l, t.code()))), top)),
    };
    let scalar = cx.slot_ty.iter().all(|t| t.is_some());
    Some(CompiledFn {
        nslots: cx.nslots,
        params,
        param_int,
        param_typed,
        ret_int,
        scalar,
        tail_hint: cx.tail_hint,
        #[cfg(feature = "jit")]
        calls: std::cell::Cell::new(0),
        #[cfg(feature = "jit")]
        jit: std::cell::OnceCell::new(),
        body,
    })
}

/// The last statement of a list: a typed integer expression, or code (`Raw`: its value may be a
/// reference, as the walker's `eval_raw` of a method call / block / island).
enum Tail {
    Int(I, IntTy),
    Code(Code),
    Raw(Code),
}

impl Tail {
    fn code(self) -> Code {
        match self {
            Tail::Int(i, t) => X::I(i, t).code(),
            Tail::Code(c) | Tail::Raw(c) => c,
        }
    }
    fn is_raw(&self) -> bool {
        matches!(self, Tail::Raw(_))
    }
}

/// A statement before the tail: code, or (in a typed body) `if c { return v }` with a typed `v`,
/// which the typed body runs as an early exit instead of unwinding a `Ctl::Return`.
enum Step {
    Code(Loc, Code),
    Guard { loc: Loc, cond: Cond, rloc: Loc, val: I, w: IntTy },
}

impl Step {
    fn code(self) -> (Loc, Code) {
        match self {
            Step::Code(l, c) => (l, c),
            Step::Guard { loc, cond, rloc, val, w } => (
                loc,
                Box::new(move |it, b| {
                    if cond(it, b)? {
                        it.mark(rloc);
                        return Err(Ctl::Return(Value::Int(val.get(it, b)? as i128, w)));
                    }
                    Ok(Value::Unit)
                }),
            ),
        }
    }
}

type Init = Vec<Step>;

/// `if c { return e }` (no `else`) as a `Step::Guard` when `e` is a typed integer.
fn guard(cx: &mut Cx, s: S) -> Option<Step> {
    let Statement::If { condition, then_block, else_block: None, .. } = s else { return None };
    let [r @ Statement::Return { value: Some(e), .. }] = then_block.as_slice() else { return None };
    if cx.needs_hint(e) || cx.boxed_ident(e) {
        return None;
    }
    let cond = expr::cond(cx, condition, "if", true)?;
    cx.scopes.push(Vec::new());
    let v = expr::expr(cx, e);
    cx.scopes.pop();
    let (loc, rloc) = (stmt_loc(s), stmt_loc(r));
    Some(match v? {
        X::I(val, w) => Step::Guard { loc, cond, rloc, val, w },
        x => {
            cx.untyped_return = true;
            let v = x.clean().code();
            Step::Code(
                loc,
                Box::new(move |it, b| {
                    if cond(it, b)? {
                        it.mark(rloc);
                        return Err(Ctl::Return(v(it, b)?));
                    }
                    Ok(Value::Unit)
                }),
            )
        }
    })
}

/// Could the walker hand `e` an expected type when a `let x: T` / `return` / tail evaluates it
/// (`expect_type_of`: calls, and `parse_string` under `?` or in a `match` block)?
fn hinted(e: E) -> bool {
    matches!(e, Expression::Call { .. } | Expression::MethodCall { .. } | Expression::TryOp { .. } | Expression::Block { .. })
}

/// A method with a `self` receiver (slot 0).
pub(super) fn has_self(f: &FuncDef) -> bool {
    f.params.first().is_some_and(|p| p.name == "self")
}

/// Does the walker set the tail's expected type at entry (`expect_tail_type`)?
fn tail_hint(f: &FuncDef) -> bool {
    f.ret.is_some() && matches!(f.body.last(), Some(Statement::Expression { expr, .. }) if hinted(expr))
}

impl Cx<'_> {
    /// Must a compiled `let x: T = e` / `return e` replay the walker's `expect_type_of(e, T)`? The
    /// hint is a single pending slot: it matters for walker code inside `e` (a block's islands), and
    /// for the tail hint set at entry, which every intervening setter clobbers. A compiled call to a
    /// non-generic function never reads it.
    fn needs_hint(&self, e: E) -> bool {
        matches!(e, Expression::Block { .. }) || (self.tail_hint && hinted(e))
    }
}

/// Every statement of a list but the last, and the last one (in value position).
fn stmt_list(cx: &mut Cx, list: &'static [S], guards: bool) -> Option<(Init, Option<(Loc, Tail)>)> {
    let Some((last, init)) = list.split_last() else { return Some((Vec::new(), None)) };
    let mut steps = Vec::with_capacity(init.len());
    for s in init {
        match if guards { guard(cx, s) } else { None } {
            Some(g) => steps.push(g),
            None => steps.push(Step::Code(stmt_loc(s), stmt(cx, s)?)),
        }
    }
    let init = steps;
    let tail = match last {
        Statement::Expression { expr: e, .. } if !cx.boxed_ident(e) => match expr::expr(cx, e) {
            Some(X::I(i, t)) => Tail::Int(i, t),
            Some(x @ X::R(_)) => Tail::Raw(x.code()),
            Some(x) => Tail::Code(x.code()),
            None => stmt_tail(cx, last)?,
        },
        s => stmt_tail(cx, s)?,
    };
    Some((init, Some((stmt_loc(last), tail))))
}

/// The walker marks every statement's location before executing it.
fn seq(init: Init, tail: Option<(Loc, Code)>) -> Code {
    let init: Vec<(Loc, Code)> = init.into_iter().map(Step::code).collect();
    let Some((lloc, lc)) = tail else { return Box::new(|_, _| Ok(Value::Unit)) };
    Box::new(move |it, b| {
        for (loc, c) in &init {
            it.mark(*loc);
            c(it, b)?;
        }
        it.mark(lloc);
        lc(it, b)
    })
}

/// A typed body; guards returning the tail's width `u` exit with their value directly.
fn typed_seq(init: Init, u: IntTy, lloc: Loc, lc: ICode) -> ICode {
    let init: Vec<Step> = init.into_iter().map(|s| match s {
        Step::Guard { w, .. } if w == u => s,
        s => {
            let (l, c) = s.code();
            Step::Code(l, c)
        }
    }).collect();
    Box::new(move |it, b| {
        for s in &init {
            match s {
                Step::Code(loc, c) => {
                    it.mark(*loc);
                    c(it, b)?;
                }
                Step::Guard { loc, cond, rloc, val, .. } => {
                    it.mark(*loc);
                    if cond(it, b)? {
                        it.mark(*rloc);
                        return val.get(it, b);
                    }
                }
            }
        }
        it.mark(lloc);
        lc(it, b)
    })
}

/// A nested block: its own name scope (slots are never reused, so no runtime work).
pub(super) fn block(cx: &mut Cx, list: &'static [S]) -> Option<Code> {
    block_x(cx, list).map(X::code)
}

/// A nested block as a value (`X::R` when its tail may yield a reference).
pub(super) fn block_x(cx: &mut Cx, list: &'static [S]) -> Option<X> {
    cx.scopes.push(Vec::new());
    let r = stmt_list(cx, list, false).map(|(init, tail)| {
        let raw = tail.as_ref().is_some_and(|(_, t)| t.is_raw());
        (seq(init, tail.map(|(l, t)| (l, t.code()))), raw)
    });
    let r = r.and_then(|(c, raw)| Some((scoped(cx, Some(c))?, raw)));
    cx.scopes.pop();
    let (c, raw) = r?;
    Some(if raw { X::R(c) } else { X::V(c) })
}

/// The walker's `pop_scope` for the innermost name scope: a panic poisons the lock guards its boxed
/// locals hold, then its locals are released (guards unlock, `Rc` counts drop) however it exits.
/// Typed slots hold plain integers and are skipped.
fn scoped(cx: &Cx, c: Option<Code>) -> Option<Code> {
    let c = c?;
    let poison = cx.boxed_in_scope();
    let clear: Vec<usize> = cx.scopes.last().map(|s| s.iter().map(|(_, i)| *i).filter(|i| cx.slot_ty(*i).is_none()).collect()).unwrap_or_default();
    if clear.is_empty() {
        return Some(c);
    }
    Some(Box::new(move |it, b| {
        let r = c(it, b);
        if is_panic(&r) {
            poison_slots(it, b, &poison);
        }
        for s in &clear {
            super::fast::set(it, b + s, Value::Unit);
        }
        r
    }))
}

/// A panic leaving the function poisons the lock guards its boxed locals hold.
fn poisoning(c: Code, slots: Vec<usize>) -> Code {
    if slots.is_empty() {
        return c;
    }
    Box::new(move |it, b| {
        let r = c(it, b);
        if is_panic(&r) {
            poison_slots(it, b, &slots);
        }
        r
    })
}

fn poisoning_i(c: ICode, slots: Vec<usize>) -> ICode {
    if slots.is_empty() {
        return c;
    }
    Box::new(move |it, b| {
        let r = c(it, b);
        if is_panic(&r) {
            poison_slots(it, b, &slots);
        }
        r
    })
}

fn is_panic<T>(r: &R<T>) -> bool {
    matches!(r, Err(Ctl::Err(e)) if e.kind == crate::interpreter::error::ErrorKind::Panic)
}

fn poison_slots(it: &mut Interpreter, b: usize, slots: &[usize]) {
    for s in slots {
        if let Value::Ref(p) = &it.rstack[b + s] {
            crate::interpreter::scope::poison_if_guard(&p.slot.borrow());
        }
    }
}

/// A statement in value position (a list's last): an `if` whose branches may yield references, or an
/// island of a value-producing statement, is `Tail::Raw`.
fn stmt_tail(cx: &mut Cx, s: S) -> Option<Tail> {
    if let Statement::If { condition, then_block, else_block, .. } = s {
        if let Some(c) = expr::cond(cx, condition, "if", true) {
            let t = block_x(cx, then_block);
            let e = match else_block {
                Some(eb) => block_x(cx, eb).map(Some),
                None => Some(None),
            };
            if let (Some(t), Some(e)) = (t, e) {
                let raw = t.is_raw() || e.as_ref().is_some_and(X::is_raw);
                let (t, e) = (t.code(), e.map(X::code));
                let code: Code = Box::new(move |it, b| match c(it, b)? {
                    true => t(it, b),
                    false => match &e {
                        Some(e) => e(it, b),
                        None => Ok(Value::Unit),
                    },
                });
                return Some(if raw { Tail::Raw(code) } else { Tail::Code(code) });
            }
        }
    }
    let native = stmt_native(cx, s);
    let island = native.is_none();
    let code = match native {
        Some(c) => c,
        None => stmt_island(cx, s)?,
    };
    let valued = matches!(s, Statement::Expression { .. } | Statement::If { .. } | Statement::Match { .. } | Statement::Loop { .. });
    // a lowered `match` yields its arm body's `eval_raw` value
    let raw = (island && valued) || matches!(s, Statement::Match { .. });
    Some(if raw { Tail::Raw(code) } else { Tail::Code(code) })
}

/// A statement: lowered when the compiler can, else a walker island (`island.rs`). Only `defer`
/// (which registers on the walker frame) is refused: the function then runs in the walker.
fn stmt(cx: &mut Cx, s: S) -> Option<Code> {
    if let Some(c) = stmt_native(cx, s) {
        return Some(c);
    }
    stmt_island(cx, s)
}

fn stmt_island(cx: &mut Cx, s: S) -> Option<Code> {
    if matches!(s, Statement::Defer { .. }) {
        return None;
    }
    if matches!(s, Statement::Return { .. }) {
        cx.untyped_return = true;
    }
    let island = Island::new(cx, &format!("{:?}", s));
    let mut names = Vec::new();
    match s {
        Statement::Let { pattern, .. } => island::pattern_names(pattern, &mut names),
        Statement::Const { name, .. } | Statement::Static { name, .. } => names.push(name.as_str()),
        _ => {}
    }
    let declares = names.into_iter().map(|n| (n, cx.declare_boxed(n))).collect();
    let island = island.declaring(declares);
    Some(Box::new(move |it, b| island.run(it, b, |it| it.exec_stmt(s))))
}

fn stmt_native(cx: &mut Cx, s: S) -> Option<Code> {
    let code: Code = match s {
        Statement::Expression { expr: e, .. } if !cx.boxed_ident(e) => expr::expr(cx, e)?.code(),
        Statement::Let { pattern, type_, value, else_block: None, .. } => {
            let name = match pattern {
                Pattern::Identifier(n) | Pattern::MutBinding(n) => Some(n.as_str()),
                Pattern::Wildcard => None,
                _ => return None,
            };
            let v = expr::expr(cx, value)?;
            let code = if v.is_raw() { raw_let(cx, name, type_.as_ref(), v) } else { let_stmt(cx, name, type_.as_ref(), v) };
            match type_ {
                Some(t) if cx.needs_hint(value) => Box::new(move |it, b| {
                    it.expect_type_of(value, t);
                    code(it, b)
                }),
                _ => code,
            }
        }
        Statement::Assignment { target, value, compound_op, .. } => {
            if let Some(c) = super::place::assign(cx, target, value, *compound_op) {
                return Some(c);
            }
            let Expression::Identifier { name, location } = target else { return None };
            let slot = cx.local(name)?;
            // boxed locals may hold `&mut` places / guards: assignments to or from them are the walker's
            if cx.is_boxed(slot) || cx.boxed_ident(value) {
                return None;
            }
            let v = expr::expr(cx, value)?;
            if v.is_raw() {
                // may store a guard / rebind to a reference: the boxed path (next compile)
                cx.need_box(slot);
                return None;
            }
            assign(cx, slot, v, *compound_op, location.as_ref())
        }
        Statement::Return { value, .. } => match value {
            Some(e) if cx.boxed_ident(e) => return None,
            Some(e) => match expr::expr(cx, e)? {
                X::I(i, t) if !(cx.ret.is_some() && cx.needs_hint(e)) => {
                    Box::new(move |it, b| Err(Ctl::Return(Value::Int(i.get(it, b)? as i128, t))))
                }
                x => {
                    cx.untyped_return = true;
                    let v = x.clean().code();
                    match cx.ret {
                        Some(rt) if cx.needs_hint(e) => Box::new(move |it, b| {
                            it.expect_type_of(e, rt);
                            Err(Ctl::Return(v(it, b)?))
                        }),
                        _ => Box::new(move |it, b| Err(Ctl::Return(v(it, b)?))),
                    }
                }
            },
            None => {
                cx.untyped_return = true;
                Box::new(|_, _| Err(Ctl::Return(Value::Unit)))
            }
        },
        Statement::If { condition, then_block, else_block, .. } => {
            let c = expr::cond(cx, condition, "if", true)?;
            let t = block(cx, then_block)?;
            let e = match else_block {
                Some(eb) => Some(block(cx, eb)?),
                None => None,
            };
            Box::new(move |it, b| match c(it, b)? {
                true => t(it, b),
                false => match &e {
                    Some(e) => e(it, b),
                    None => Ok(Value::Unit),
                },
            })
        }
        Statement::While { condition, body, label, .. } => {
            let c = expr::cond(cx, condition, "while", false)?;
            let body = block(cx, body)?;
            let label: Option<&'static str> = label.as_deref();
            Box::new(move |it, b| {
                while c(it, b)? {
                    if loop_exit(body(it, b), label)?.is_some() {
                        break;
                    }
                }
                Ok(Value::Unit)
            })
        }
        Statement::Loop { body, label, .. } => {
            let body = block(cx, body)?;
            let label: Option<&'static str> = label.as_deref();
            Box::new(move |it, b| loop {
                if let Some(v) = loop_exit(body(it, b), label)? {
                    return Ok(v);
                }
            })
        }
        Statement::For { pattern, iterable, body, label, .. } => for_stmt(cx, pattern, iterable, body, label.as_deref())?,
        Statement::Break { label, value, .. } => {
            let v = match value {
                Some(e) => Some(expr::expr(cx, e)?.clean().code()),
                None => None,
            };
            let label = label.clone();
            Box::new(move |it, b| {
                let value = match &v {
                    Some(v) => v(it, b)?,
                    None => Value::Unit,
                };
                Err(Ctl::Break(Box::new((label.clone(), value))))
            })
        }
        Statement::Continue { label, .. } => {
            let label = label.clone();
            Box::new(move |_, _| Err(Ctl::Continue(Box::new(label.clone()))))
        }
        Statement::Use { .. } => Box::new(|_, _| Ok(Value::Unit)),
        Statement::Match { value, arms, .. } => super::matching::match_stmt(cx, value, arms)?,
        _ => return None,
    };
    Some(code)
}

/// `let x = <method call / block>` whose value may be a reference (the walker's `eval_raw_binding`):
/// the local is boxed (reads follow references) and the declared type coerces anything but a reference.
fn raw_let(cx: &mut Cx, name: Option<&'static str>, ty: Option<&'static Type>, v: X) -> Code {
    let v = match name {
        Some(_) => v.code(),
        None => v.clean().code(), // `let _ = e` evaluates with `eval`
    };
    let slot = name.map(|n| cx.declare_boxed(n));
    Box::new(move |it, b| {
        let mut x = v(it, b)?;
        if let Some(t) = ty {
            if !matches!(x, Value::Ref(_)) {
                x = it.coerce(x, t)?;
            }
        }
        if let Some(s) = slot {
            super::fast::set(it, b + s, island::boxed(x));
        }
        Ok(Value::Unit)
    })
}

/// `let [name][: ty] = v`: a typed integer value (with an integer or no annotation) makes a typed slot.
fn let_stmt(cx: &mut Cx, name: Option<&'static str>, ty: Option<&'static Type>, v: X) -> Code {
    let width = match (&v, ty.map(IntTy::from_type)) {
        (X::I(_, u), None) => Some(*u),
        (X::I(..), Some(Some(t))) if small(t) => Some(t),
        _ => None,
    };
    if let (Some(t), X::I(..)) = (width, &v) {
        let X::I(i, u) = v else { unreachable!() };
        let slot = name.map(|n| cx.declare(n, Some(t)));
        let kind = slot_kind(cx, slot);
        return Box::new(move |it, b| {
            let mut x = i.get(it, b)?;
            if let Some(ty) = ty {
                x = typed::retype(it, x, u, t, ty)?;
            }
            match (slot, kind) {
                (Some(s), Kind::Typed) => typed::set_int(it, b + s, x, t),
                (Some(s), k) => store(it, b + s, Value::Int(x as i128, t), k),
                (None, _) => {}
            }
            Ok(Value::Unit)
        });
    }
    let v = v.code();
    let slot = name.map(|n| cx.declare(n, None));
    let kind = slot_kind(cx, slot);
    Box::new(move |it, b| {
        let mut x = v(it, b)?;
        if let Some(t) = ty {
            x = it.coerce(x, t)?;
        }
        if let Some(s) = slot {
            store(it, b + s, x, kind);
        }
        Ok(Value::Unit)
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Plain,
    Typed,
    Boxed,
}

fn slot_kind(cx: &Cx, slot: Option<usize>) -> Kind {
    match slot {
        Some(s) if cx.is_boxed(s) => Kind::Boxed,
        Some(s) if cx.slot_ty(s).is_some() => Kind::Typed,
        _ => Kind::Plain,
    }
}

/// A fresh binding of a slot (`let`, a loop variable): a boxed local gets a new walker slot, as the
/// walker's `define` does (closures that captured the previous one keep it).
#[inline(always)]
fn store(it: &mut Interpreter, i: usize, v: Value, kind: Kind) {
    match kind {
        Kind::Boxed => super::fast::set(it, i, island::boxed(v)),
        _ => super::fast::set(it, i, v),
    }
}

/// `slot = v` / `slot op= v`; the walker evaluates the right side, marks the target, then stores.
fn assign(cx: &mut Cx, slot: usize, v: X, op: Option<CompoundOp>, tloc: Loc) -> Code {
    let bop = op.map(crate::interpreter::ops::compound_to_binary);
    let rhs = match &v {
        X::I(I::Const(k), u) => Some((*u, Some(*k))),
        X::I(_, u) => Some((*u, None)),
        _ => None,
    };
    if let Some(t) = cx.slot_ty(slot) {
        if cx.assign_ok(slot, rhs, bop) {
            let X::I(i, u) = v else { unreachable!("assign_ok accepts typed integers only") };
            return match bop {
                None => Box::new(move |it, b| {
                    let x = i.get(it, b)?;
                    it.mark(tloc);
                    typed::set_int(it, b + slot, x, t);
                    Ok(Value::Unit)
                }),
                Some(bop) => {
                    let (code, _) = typed::arith(bop, I::Slot(slot), t, i, u, tloc).expect("integer operator");
                    Box::new(move |it, b| {
                        let x = code(it, b)?;
                        typed::set_int(it, b + slot, x, t);
                        Ok(Value::Unit)
                    })
                }
            };
        }
    }
    untyped_assign(slot, v.code(), op, tloc)
}

fn untyped_assign(slot: usize, v: Code, op: Option<CompoundOp>, tloc: Loc) -> Code {
    Box::new(move |it, b| {
        let rhs = v(it, b)?;
        it.mark(tloc);
        let new = match op {
            None => retag(&it.rstack[b + slot], rhs),
            Some(cop) => {
                let bop = crate::interpreter::ops::compound_to_binary(cop);
                if let Some(r) = super::fast::compound_int(it, b, slot, bop, &rhs) {
                    return r.map(|_| Value::Unit);
                }
                let cur = std::mem::replace(&mut it.rstack[b + slot], Value::Unit);
                if let (Value::Str(_), Value::Str(add), CompoundOp::Add) = (&cur, &rhs, cop) {
                    let mut cur = cur;
                    if let Value::Str(rc) = &mut cur {
                        Rc::make_mut(rc).push_str(add);
                    }
                    it.rstack[b + slot] = cur;
                    return Ok(Value::Unit);
                }
                match it.binary(bop, cur.clone(), rhs) {
                    Ok(x) => x,
                    Err(e) => {
                        it.rstack[b + slot] = cur;
                        return Err(e);
                    }
                }
            }
        };
        super::fast::set(it, b + slot, new);
        Ok(Value::Unit)
    })
}

/// The walker's `retag`: assigning a bare literal keeps the declared width of the old value.
fn retag(old: &Value, new: Value) -> Value {
    match (old, &new) {
        (Value::Int(_, ot), Value::Int(n, IntTy::Lit)) if *ot != IntTy::Lit && ot.fits(*n) => Value::Int(*n, *ot),
        (Value::Float(_, ot), Value::Float(n, FloatTy::Lit)) => Value::Float(*n, *ot),
        (Value::Float(_, ot), Value::Int(n, IntTy::Lit)) => Value::Float(*n as f64, *ot),
        _ => new,
    }
}

/// The loop variable's slot (in a new scope the caller pops), or `Err(())` for an unsupported pattern.
fn loop_var(cx: &mut Cx, pattern: &'static Pattern<'static>, ty: Option<IntTy>) -> Result<Option<usize>, ()> {
    cx.scopes.push(Vec::new());
    match pattern {
        Pattern::Identifier(n) | Pattern::MutBinding(n) => Ok(Some(cx.declare(n.as_str(), ty))),
        Pattern::Wildcard => Ok(None),
        _ => {
            cx.scopes.pop();
            Err(())
        }
    }
}

fn for_stmt(cx: &mut Cx, pattern: &'static Pattern<'static>, iterable: E, body: &'static [S], label: Option<&'static str>) -> Option<Code> {
    // the walker iterates a reference by reference (`for x in &mut v`, a `&mut` parameter)
    if cx.boxed_ident(iterable) {
        return None;
    }
    let src = match iterable {
        Expression::Range { start, end, inclusive, .. } => match (expr::expr(cx, start)?, expr::expr(cx, end)?) {
            // `for i in a..b` over typed bounds: count in place, the variable is a typed slot
            (X::I(s, st), X::I(e, et)) => return typed_for(cx, (s, e, *inclusive, IntTy::unify(st, et)), pattern, body, label),
            (s, e) => expr::range_code(s.clean().code(), e.clean().code(), *inclusive),
        },
        _ => match expr::expr(cx, iterable)? {
            // a reference is iterated by reference: the walker's
            X::R(_) => return None,
            x => x.code(),
        },
    };
    let slot = loop_var(cx, pattern, None).ok()?;
    let kind = slot_kind(cx, slot);
    let body = block(cx, body);
    let body = scoped(cx, body);
    cx.scopes.pop();
    let body = body?;
    Some(Box::new(move |it, b| {
        let v = src(it, b)?;
        if let Value::Range(lo, hi, ty) = v {
            let mut i = lo;
            while i < hi {
                if let Some(s) = slot {
                    store(it, b + s, Value::Int(i, ty), kind);
                }
                if loop_exit(body(it, b), label)?.is_some() {
                    break;
                }
                i += 1;
            }
            return Ok(Value::Unit);
        }
        let iter = crate::interpreter::iter::as_iter(it, v)?;
        while let Some(x) = crate::interpreter::iter::next(it, &iter)? {
            if let Some(s) = slot {
                store(it, b + s, x, kind);
            }
            if loop_exit(body(it, b), label)?.is_some() {
                break;
            }
        }
        Ok(Value::Unit)
    }))
}

fn typed_for(cx: &mut Cx, range: (I, I, bool, IntTy), pattern: &'static Pattern<'static>, body: &'static [S], label: Option<&'static str>) -> Option<Code> {
    let (s, e, inclusive, ty) = range;
    let slot = loop_var(cx, pattern, Some(ty)).ok()?;
    let kind = slot_kind(cx, slot);
    let body = block(cx, body);
    let body = scoped(cx, body);
    cx.scopes.pop();
    let body = body?;
    Some(Box::new(move |it, b| {
        let lo = s.get(it, b)? as i128;
        let hi = e.get(it, b)? as i128 + inclusive as i128;
        let mut i = lo;
        while i < hi {
            match slot {
                Some(s) if kind == Kind::Typed => typed::set_int(it, b + s, i as i64, ty),
                Some(s) => store(it, b + s, Value::Int(i, ty), kind),
                None => {}
            }
            if loop_exit(body(it, b), label)?.is_some() {
                break;
            }
            i += 1;
        }
        Ok(Value::Unit)
    }))
}

pub(super) fn run_all(it: &mut Interpreter, b: usize, codes: &[Code]) -> R<Vec<Value>> {
    let mut out = Vec::with_capacity(codes.len());
    for c in codes {
        out.push(c(it, b)?);
    }
    Ok(out)
}
