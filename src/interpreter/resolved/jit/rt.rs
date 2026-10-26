//! Runtime helpers machine-code loops call for what lives in `Value`s: elements of a `Vec`, fields
//! of a struct, `push`, `len`, and integer width combinations without an inline fast path.
//!
//! Each helper takes the happy path or reports failure (`Ctx::fail`) before it changes anything;
//! the loop then stops at the statement and the closure engine runs it (and produces the walker's
//! panic, if there is one). Stores follow the walker's `Place::with_mut` (copy-on-write along the
//! path, `retag` for `=`); reads follow references without cloning.

use super::kinds::{bits, from_code, int_value, mutant, value, Ty};
use crate::interpreter::engine::Interpreter;
use crate::interpreter::eval_stmt::retag;
use crate::interpreter::ops::int_binary;
use crate::interpreter::place::descend;
use crate::interpreter::value::{Proj, Value};
use crate::parser::BinaryOp;
use std::rc::Rc;

/// What machine code passes every helper: the interpreter, the frame base, and the failure flag.
#[repr(C)]
pub(crate) struct Ctx {
    pub(crate) it: *mut Interpreter,
    pub(crate) base: usize,
    pub(crate) fail: i64,
    /// The running unit's plan and registry (for closure-run statements).
    pub(crate) plan: *const super::lower::Plan,
    pub(crate) reg: *const crate::interpreter::resolved::osr::Reg,
    /// What a closure-run statement raised (the unit then leaves after it).
    pub(crate) raised: Option<crate::interpreter::error::Ctl>,
}

pub(crate) const FAIL: i32 = std::mem::offset_of!(Ctx, fail) as i32;

/// Run `f` with the local at `slot`; a `None` sets the failure flag.
unsafe fn with_slot(c: *mut Ctx, slot: i64, f: impl FnOnce(&mut Value) -> Option<i64>) -> i64 {
    // SAFETY: `c` is the context `osr::enter` built for this call; the interpreter outlives it and
    // nothing else touches it while machine code runs.
    let ctx = unsafe { &mut *c };
    let it = unsafe { &mut *ctx.it };
    let i = ctx.base + slot as usize;
    match it.rstack.get_mut(i).and_then(f) {
        Some(x) => x,
        None => {
            ctx.fail = 1;
            0
        }
    }
}

/// Read what `v` holds, following references.
pub(crate) fn peek<T>(v: &Value, f: impl FnOnce(&Value) -> Option<T>) -> Option<T> {
    match v {
        Value::Ref(p) => {
            let g = p.slot.try_borrow().ok()?;
            let mut cur: &Value = &g;
            for pr in &p.path {
                cur = step(cur, pr)?;
            }
            peek(cur, f)
        }
        Value::Rc(_) => None,
        v => f(v),
    }
}

fn step<'v>(v: &'v Value, p: &Proj) -> Option<&'v Value> {
    match (v, p) {
        (Value::Struct(s), Proj::Field(n)) => s.fields.iter().find(|(m, _)| **m == **n).map(|(_, v)| v),
        (Value::Vec(xs), Proj::Index(i)) => xs.get(*i),
        (Value::Tuple(t), Proj::Index(i)) => t.get(*i),
        _ => None,
    }
}

/// Mutate what `v` holds, following references (the walker's `Place::with_mut`).
fn poke<T>(v: &mut Value, f: impl FnOnce(&mut Value) -> Option<T>) -> Option<T> {
    match v {
        Value::Ref(p) => {
            let mut g = p.slot.try_borrow_mut().ok()?;
            let mut cur: &mut Value = &mut g;
            for pr in &p.path {
                cur = descend(cur, pr).ok()?;
            }
            poke(cur, f)
        }
        Value::Rc(_) => None,
        v => f(v),
    }
}

/// An index value (`i` at type code `t`) as the walker checks it: negative is out of bounds.
fn index(i: i64, t: i64) -> Option<usize> {
    match from_code(t)? {
        Ty::Int(t) => usize::try_from(int_value(i, t)).ok(),
        _ => None,
    }
}

fn name(p: i64, n: i64) -> &'static str {
    // SAFETY: `p`/`n` are the pointer and length of a field name in the program's AST, which lives
    // for the whole run.
    unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(p as *const u8, n as usize)) }
}

/// A field name as machine code names it: the text, the position it had at entry and the address
/// of the interned name it had there (equal addresses are equal names: no text comparison).
#[derive(Clone, Copy)]
struct FieldRef {
    name: &'static str,
    hint: usize,
    id: usize,
}

impl FieldRef {
    fn new(p: i64, n: i64, hint: i64, id: i64) -> FieldRef {
        FieldRef { name: name(p, n), hint: hint as usize, id: id as usize }
    }
    fn is(&self, n: &Rc<str>) -> bool {
        n.as_ptr() as usize == self.id || **n == *self.name
    }
}

fn field(v: &Value, f: FieldRef) -> Option<&Value> {
    let Value::Struct(s) = v else { return None };
    match s.fields.get(f.hint) {
        Some((n, x)) if f.is(n) => Some(x),
        _ => s.fields.iter().find(|(n, _)| **n == *f.name).map(|(_, x)| x),
    }
}

fn field_mut(v: &mut Value, f: FieldRef) -> Option<&mut Value> {
    let Value::Struct(s) = v else { return None };
    let s = Rc::make_mut(s);
    let i = match s.fields.get(f.hint) {
        Some((n, _)) if f.is(n) => f.hint,
        _ => s.fields.iter().position(|(n, _)| **n == *f.name)?,
    };
    Some(&mut s.fields[i].1)
}

/// `=` into `slot` (the walker's `retag`: a literal keeps the old value's width).
fn store(slot: &mut Value, x: i64, t: i64) -> Option<i64> {
    let new = value(x, from_code(t)?);
    let old = std::mem::replace(slot, Value::Unit);
    *slot = retag(old, new);
    Some(0)
}

/// `v[i]` (an element of type `want`).
pub(crate) unsafe extern "C" fn index_get(c: *mut Ctx, slot: i64, i: i64, it: i64, want: i64) -> i64 {
    // SAFETY: as in `with_slot`.
    let off_by_one = unsafe { mutant(&*(*c).it, "bounds") };
    unsafe {
        with_slot(c, slot, |v| {
            let (k, want) = (index(i, it)?, from_code(want)?);
            peek(v, |v| match v {
                Value::Vec(xs) if off_by_one => bits(xs.get(k).or(xs.last())?, want),
                Value::Vec(xs) => bits(xs.get(k)?, want),
                _ => None,
            })
        })
    }
}

/// `v[i].f`.
pub(crate) unsafe extern "C" fn index_field_get(c: *mut Ctx, slot: i64, i: i64, it: i64, fp: i64, fl: i64, hint: i64, id: i64, want: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            let (k, want, f) = (index(i, it)?, from_code(want)?, FieldRef::new(fp, fl, hint, id));
            peek(v, |v| match v {
                Value::Vec(xs) => bits(field(xs.get(k)?, f)?, want),
                _ => None,
            })
        })
    }
}

/// `s.f`.
pub(crate) unsafe extern "C" fn field_get(c: *mut Ctx, slot: i64, fp: i64, fl: i64, hint: i64, id: i64, want: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            let (want, f) = (from_code(want)?, FieldRef::new(fp, fl, hint, id));
            peek(v, |v| bits(field(v, f)?, want))
        })
    }
}

/// `v[i] = x`.
pub(crate) unsafe extern "C" fn index_set(c: *mut Ctx, slot: i64, i: i64, it: i64, x: i64, xt: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            let k = index(i, it)?;
            poke(v, |v| match v {
                Value::Vec(xs) if k < xs.len() => store(&mut Rc::make_mut(xs)[k], x, xt),
                _ => None,
            })
        })
    }
}

/// `v[i].f = x`.
pub(crate) unsafe extern "C" fn index_field_set(c: *mut Ctx, slot: i64, i: i64, it: i64, fp: i64, fl: i64, hint: i64, id: i64, x: i64, xt: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            let (k, f) = (index(i, it)?, FieldRef::new(fp, fl, hint, id));
            poke(v, |v| match v {
                // the field must exist before anything is made unique (as `descend` checks it)
                Value::Vec(xs) if k < xs.len() && field(&xs[k], f).is_some() => store(field_mut(&mut Rc::make_mut(xs)[k], f)?, x, xt),
                _ => None,
            })
        })
    }
}

/// `s.f = x`.
pub(crate) unsafe extern "C" fn field_set(c: *mut Ctx, slot: i64, fp: i64, fl: i64, hint: i64, id: i64, x: i64, xt: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            let f = FieldRef::new(fp, fl, hint, id);
            poke(v, |v| {
                field(v, f)?;
                store(field_mut(v, f)?, x, xt)
            })
        })
    }
}

/// `v.push(x)`.
pub(crate) unsafe extern "C" fn push(c: *mut Ctx, slot: i64, x: i64, xt: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            let x = value(x, from_code(xt)?);
            poke(v, |v| match v {
                Value::Vec(xs) => {
                    Rc::make_mut(xs).push(x);
                    Some(0)
                }
                _ => None,
            })
        })
    }
}

/// `v.len()` of a `Vec`.
pub(crate) unsafe extern "C" fn len(c: *mut Ctx, slot: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            peek(v, |v| match v {
                Value::Vec(xs) => Some(xs.len() as i64),
                _ => None,
            })
        })
    }
}

pub(crate) const OPS: [BinaryOp; 8] =
    [BinaryOp::Add, BinaryOp::Sub, BinaryOp::Mul, BinaryOp::Div, BinaryOp::Mod, BinaryOp::BitAnd, BinaryOp::BitOr, BinaryOp::BitXor];

/// `a op b` for width combinations without an inline path: the walker's `int_binary`.
pub(crate) unsafe extern "C" fn int_op(c: *mut Ctx, op: i64, a: i64, at: i64, b: i64, bt: i64, want: i64) -> i64 {
    let r = (|| {
        let (Ty::Int(at), Ty::Int(bt)) = (from_code(at)?, from_code(bt)?) else { return None };
        let v = int_binary(*OPS.get(op as usize)?, int_value(a, at), at, int_value(b, bt), bt).ok()?;
        bits(&v, from_code(want)?)
    })();
    match r {
        Some(x) => x,
        None => {
            // SAFETY: as in `with_slot`.
            unsafe { (*c).fail = 1 };
            0
        }
    }
}

/// Run closure-run statement `k` of the plan: its variables go to their slots, its code runs (marked
/// as `seq` marks it), and they come back to the spill buffer. Nonzero: leave after the statement (it
/// raised, `raised` holds what; or a variable no longer has its type).
pub(crate) unsafe extern "C" fn run_stmt(c: *mut Ctx, buf: i64, k: i64) -> i64 {
    // SAFETY: as in `with_slot`; `plan`/`reg` outlive the call, `buf` has `nbuf` slots.
    let ctx = unsafe { &mut *c };
    let (it, plan, reg, b) = unsafe { (&mut *ctx.it, &*ctx.plan, &*ctx.reg, ctx.base) };
    let buf = buf as *mut i64;
    let st = &plan.stmts[k as usize];
    let at = |i: usize| unsafe { buf.add(i) };
    for sp in st.spills.iter().filter(|sp| sp.slot != super::lower::NO_SLOT) {
        let v = value(unsafe { *at(sp.buf) }, sp.ty);
        super::osr::write(it, b + sp.slot, reg.kind(sp.slot), v, false);
    }
    it.mark(crate::interpreter::resolved::osr::loc(st.stmt));
    let r = match reg.code(st.stmt) {
        Some(code) => code(it, b),
        None => Err(crate::interpreter::error::semantic("internal: a machine-code loop ran an unregistered statement")),
    };
    if let Err(e) = r {
        ctx.raised = Some(e);
        return 1;
    }
    for sp in st.spills.iter().filter(|sp| sp.slot != super::lower::NO_SLOT) {
        match super::osr::read(it, b + sp.slot, reg.kind(sp.slot)).and_then(|v| bits(&v, sp.ty)) {
            Some(x) => unsafe { *at(sp.buf) = x },
            None => return 1,
        }
    }
    0
}

pub(crate) extern "C" fn fmod(a: f64, b: f64) -> f64 {
    a % b
}

pub(crate) extern "C" fn round(x: f64) -> f64 {
    x.round()
}
