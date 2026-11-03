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
    /// Locals reached through references, resolved to the cell that holds their value (`with_cell`): the chain
    /// is walked once per unit run instead of on every element access. Cleared before every closure-run
    /// statement, the only statements that can rebind a reference.
    pub(crate) cells: Vec<(i64, crate::interpreter::value::Slot)>,
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
    let direct_off = unsafe { (*(*c).it).fast_off("packedidx") };
    let mut direct = false;
    let r = unsafe {
        with_cell(c, slot, |v| {
            let (k, want) = (index(i, it)?, from_code(want)?);
            match v {
                Value::Vec(xs) if off_by_one => bits(&xs.get_value(k).or_else(|| xs.get_value(xs.len().checked_sub(1)?))?, want),
                Value::Vec(xs) => match packed_get(xs, k, want).filter(|_| !direct_off) {
                    Some(r) => {
                        direct = true;
                        r
                    }
                    None => bits(&xs.get_value(k)?, want),
                },
                _ => None,
            }
        })
    };
    if direct {
        unsafe { (*(*c).it).fast.packedidx += 1 };
    }
    r
}

/// `v[i].f`.
pub(crate) unsafe extern "C" fn index_field_get(c: *mut Ctx, slot: i64, i: i64, it: i64, fp: i64, fl: i64, hint: i64, id: i64, want: i64) -> i64 {
    unsafe {
        with_cell(c, slot, |v| {
            let (k, want, f) = (index(i, it)?, from_code(want)?, FieldRef::new(fp, fl, hint, id));
            match v {
                Value::Vec(xs) => bits(field(xs.get(k)?, f)?, want),
                _ => None,
            }
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
    // SAFETY: as in `with_slot`; the interpreter is not borrowed across the call below.
    let direct_off = unsafe { (*(*c).it).fast_off("packedidx") };
    let mut direct = false;
    let r = unsafe {
        with_cell_mut(c, slot, |v| {
            let k = index(i, it)?;
            let t = from_code(xt)?;
            match v {
                Value::Vec(xs) => {
                    if !direct_off && packed_set(xs, k, x, t) {
                        direct = true;
                        return Some(0);
                    }
                    if k >= xs.len() {
                        return None;
                    }
                    // the walker's `retag` against the old element, then a plain replacement
                    let new = retag(xs.get_value(k)?, value(x, t));
                    Rc::make_mut(xs).set_value(k, new);
                    Some(0)
                }
                _ => None,
            }
        })
    };
    if direct {
        unsafe { (*(*c).it).fast.packedidx += 1 };
    }
    r
}

/// `v[i].f = x`.
pub(crate) unsafe extern "C" fn index_field_set(c: *mut Ctx, slot: i64, i: i64, it: i64, fp: i64, fl: i64, hint: i64, id: i64, x: i64, xt: i64) -> i64 {
    unsafe {
        with_cell_mut(c, slot, |v| {
            let (k, f) = (index(i, it)?, FieldRef::new(fp, fl, hint, id));
            match v {
                // the field must exist before anything is made unique (as `descend` checks it)
                Value::Vec(xs) if k < xs.len() && field(&xs[k], f).is_some() => store(field_mut(&mut Rc::make_mut(xs)[k], f)?, x, xt),
                _ => None,
            }
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
    // SAFETY: as in `with_slot`; the interpreter is not borrowed across the call below.
    let off = unsafe { (*(*c).it).fast_off("packed") };
    let mut packed = false;
    let r = unsafe {
        with_slot(c, slot, |v| {
            let x = value(x, from_code(xt)?);
            poke(v, |v| match v {
                Value::Vec(xs) => {
                    let body = Rc::make_mut(xs);
                    if off {
                        body.push(x);
                    } else {
                        body.push_value(x);
                        packed = body.packed().is_some();
                    }
                    Some(0)
                }
                _ => None,
            })
        })
    };
    if packed {
        unsafe { (*(*c).it).fast.packed += 1 };
    }
    r
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
    // closure code may replace what a slot holds (`Ctx::cells`): resolve again after it. Assigning a new
    // reference to a reference variable writes through it today, so no corpus program can make a
    // resolution stale; this keeps the cache safe if that changes.
    ctx.cells.clear();
    let (it, plan, reg, b) = unsafe { (&mut *ctx.it, &*ctx.plan, &*ctx.reg, ctx.base) };
    let buf = buf as *mut i64;
    let st = &plan.stmts[k as usize];
    let at = |i: usize| unsafe { buf.add(i) };
    for sp in st.spills.iter().filter(|sp| sp.slot != super::lower::NO_SLOT) {
        let v = value(unsafe { *at(sp.buf) }, sp.ty);
        super::osr::write(it, b + sp.slot, reg.kind(sp.slot), v, false);
    }
    {
        // SAFETY: `buf` has `nbuf` slots, every struct field index is one of them.
        let all = unsafe { std::slice::from_raw_parts_mut(buf, plan.nbuf.max(1)) };
        // test-only mutant `box`: the first write after a struct `let` fills the previous binding's box
        let reuse = mutant(it, "box");
        for sp in &st.structs {
            super::osr::write(it, b + sp.slot, reg.kind(sp.slot), super::osr::make_struct(sp, all), sp.fresh && !reuse);
        }
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
    let all = unsafe { std::slice::from_raw_parts_mut(buf, plan.nbuf.max(1)) };
    for sp in &st.structs {
        if super::osr::load_struct(it, b + sp.slot, reg.kind(sp.slot), sp, all).is_none() {
            return 1;
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

/// `v[k]` read straight from a packed body holding exactly type `want` (`Some(None)`: out of bounds,
/// which fails as the element read does); `None`: not packed with that type, the element path decides.
/// The packing holds what `bits` would make of the element.
fn packed_get(xs: &crate::interpreter::vecbody::VecBody, k: usize, want: Ty) -> Option<Option<i64>> {
    use crate::interpreter::vecbody::Packed;
    Some(match (xs.packed()?, want) {
        (Packed::Bool(v), Ty::Bool) => v.get(k).map(|b| *b as i64),
        (Packed::Int(v, t), Ty::Int(w)) if *t == w => v.get(k).copied(),
        (Packed::Float(v, t), Ty::Flt(w)) if *t == w => v.get(k).map(|f| f.to_bits() as i64),
        _ => return None,
    })
}

/// `v[k] = x` written straight into a packed body when `x` (bits at type `xt`) has exactly the packing's
/// type: then the walker's `retag` keeps `x` as it is and the plain replacement packs it. `false`: the
/// element path decides (another type, a literal width to retag, an unpacked body).
fn packed_set(xs: &mut Rc<crate::interpreter::vecbody::VecBody>, k: usize, x: i64, xt: Ty) -> bool {
    use crate::interpreter::vecbody::{Packed, VecBody};
    let fits = matches!((xs.packed(), xt), (Some(Packed::Bool(_)), Ty::Bool))
        || matches!((xs.packed(), xt), (Some(Packed::Int(_, t)), Ty::Int(w)) if *t == w)
        || matches!((xs.packed(), xt), (Some(Packed::Float(_, t)), Ty::Flt(w)) if *t == w);
    if !fits || k >= xs.len() {
        return false;
    }
    let VecBody::Packed(p, cache) = Rc::make_mut(xs) else { return false };
    match (p, xt) {
        (Packed::Bool(v), _) => v[k] = x != 0,
        (Packed::Int(v, _), Ty::Int(t)) => v[k] = int_value(x, t) as i64,
        (Packed::Float(v, _), _) => v[k] = f64::from_bits(x as u64),
        _ => return false,
    }
    cache.take();
    true
}

/// The cell a local's value lives in when the local reaches it through plain references (no projection,
/// no lock guard), resolved once per unit run; `None`: the local holds its value directly, or the chain
/// has a projection (the element path walks it as before).
unsafe fn local_cell(c: *mut Ctx, slot: i64) -> Option<crate::interpreter::value::Slot> {
    // SAFETY: as in `with_slot`.
    let ctx = unsafe { &mut *c };
    if let Some((_, cell)) = ctx.cells.iter().find(|(s, _)| *s == slot) {
        return Some(cell.clone());
    }
    let it = unsafe { &*ctx.it };
    if it.fast_off("vecresolve") {
        return None;
    }
    let Value::Ref(p) = it.rstack.get(ctx.base + slot as usize)? else { return None };
    if !p.path.is_empty() || p.guard.is_some() {
        return None;
    }
    let mut cell = p.slot.clone();
    loop {
        let next = match &*cell.try_borrow().ok()? {
            Value::Ref(q) if q.path.is_empty() && q.guard.is_none() => q.slot.clone(),
            Value::Ref(_) => return None,
            _ => break,
        };
        cell = next;
    }
    ctx.cells.push((slot, cell.clone()));
    Some(cell)
}

/// `with_slot` for a read of a local that may sit behind references: through its resolved cell when it has one.
unsafe fn with_cell(c: *mut Ctx, slot: i64, f: impl FnOnce(&Value) -> Option<i64>) -> i64 {
    match unsafe { local_cell(c, slot) } {
        Some(cell) => {
            // SAFETY: as in `with_slot`.
            let ctx = unsafe { &mut *c };
            unsafe { (*ctx.it).fast.vecresolve += 1 };
            match cell.try_borrow().ok().and_then(|g| f(&g)) {
                Some(x) => x,
                None => {
                    ctx.fail = 1;
                    0
                }
            }
        }
        None => unsafe { with_slot(c, slot, |v| peek(v, f)) },
    }
}

/// `with_slot` for a write through a local that may sit behind references (see `with_cell`).
unsafe fn with_cell_mut(c: *mut Ctx, slot: i64, f: impl FnOnce(&mut Value) -> Option<i64>) -> i64 {
    match unsafe { local_cell(c, slot) } {
        Some(cell) => {
            // SAFETY: as in `with_slot`.
            let ctx = unsafe { &mut *c };
            unsafe { (*ctx.it).fast.vecresolve += 1 };
            match cell.try_borrow_mut().ok().and_then(|mut g| f(&mut g)) {
                Some(x) => x,
                None => {
                    ctx.fail = 1;
                    0
                }
            }
        }
        None => unsafe { with_slot(c, slot, |v| poke(v, f)) },
    }
}
