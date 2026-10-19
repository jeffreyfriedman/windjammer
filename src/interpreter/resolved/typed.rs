//! Statically typed integer code (ADR-0020 step 1): an expression whose integer width is known at
//! compile time compiles to an `ICode` that yields a raw `i64` -- no `Value` is built, matched or
//! dropped on the hot path. Only widths whose every value fits an `i64` qualify (`small`); `u64`,
//! `usize`, `i128` and `u128` stay on the `Value` path. Every edge (overflow, a zero divisor,
//! shifts, out-of-range literals) is delegated to the walker's `int_binary` / `coerce`, so results
//! and panic messages are the walker's.

use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::{panic_ctl, semantic, R};
use crate::interpreter::int::IntTy;
use crate::interpreter::ops::int_binary;
use crate::interpreter::value::Value;
use crate::parser::BinaryOp;
use crate::source_map::Location;

pub(crate) type ICode = Box<dyn Fn(&mut Interpreter, usize) -> R<i64>>;
type Loc = Option<&'static Location>;

/// Widths whose values all fit an `i64` (and so are carried unboxed).
pub(super) fn small(t: IntTy) -> bool {
    !matches!(t, IntTy::U64 | IntTy::Usize | IntTy::I128 | IntTy::U128)
}

/// A typed integer operand: a typed local slot, a constant, or code.
pub(super) enum I {
    Slot(usize),
    Const(i64),
    Code(ICode),
}

impl I {
    #[inline(always)]
    pub(super) fn get(&self, it: &mut Interpreter, b: usize) -> R<i64> {
        match self {
            I::Slot(s) => Ok(slot_int(it, b + s)),
            I::Const(c) => Ok(*c),
            I::Code(c) => c(it, b),
        }
    }

    pub(super) fn into_code(self) -> ICode {
        match self {
            I::Code(c) => c,
            I::Slot(s) => Box::new(move |it, b| Ok(slot_int(it, b + s))),
            I::Const(c) => Box::new(move |_, _| Ok(c)),
        }
    }
}

/// A typed slot always holds a `Value::Int` that fits an `i64` (see `compile.rs`, slot typing).
#[inline(always)]
pub(super) fn slot_int(it: &Interpreter, i: usize) -> i64 {
    match &it.rstack[i] {
        Value::Int(a, _) => *a as i64,
        _ => typed_slot_broken(),
    }
}

#[cold]
#[inline(never)]
fn typed_slot_broken() -> i64 {
    panic!("resolved engine: a typed slot holds a non-integer (slot typing bug)")
}

/// Store into a typed slot (its old value is an integer or `Unit`: nothing to drop).
#[inline(always)]
pub(super) fn set_int(it: &mut Interpreter, i: usize, v: i64, t: IntTy) {
    std::mem::forget(std::mem::replace(&mut it.rstack[i], Value::Int(v as i128, t)));
}

/// A `Value` known (by declaration) to be an integer of width `t`; anything else is a type error
/// the program would not have compiled with.
#[inline(always)]
pub(super) fn unbox(v: Value, t: IntTy) -> R<i64> {
    match v {
        Value::Int(a, vt) if vt == t && a as i64 as i128 == a => Ok(a as i64),
        other => Err(mismatch(t, &other)),
    }
}

#[cold]
#[inline(never)]
fn mismatch(t: IntTy, v: &Value) -> crate::interpreter::error::Ctl {
    semantic(format!("mismatched types: expected `{}`, found `{}`", t.name(), v.type_name()))
}

/// `v` of width `from` bound to a declared `to` (`coerce`): literals are range-checked, the rest retag.
#[inline(always)]
pub(super) fn retype(it: &mut Interpreter, v: i64, from: IntTy, to: IntTy, ty: &crate::parser::Type) -> R<i64> {
    if from == IntTy::Lit && !to.fits(v as i128) {
        return it.coerce(Value::Int(v as i128, from), ty).and_then(|x| unbox(x, to));
    }
    Ok(v)
}

/// Bounds of a small width as `i64`; `None` when it is the whole `i64` range (no check needed).
fn bounds(t: IntTy) -> Option<(i64, i64)> {
    let (lo, hi) = (t.min() as i64, t.max() as i64);
    (lo != i64::MIN || hi != i64::MAX).then_some((lo, hi))
}

#[cold]
#[inline(never)]
fn slow(op: BinaryOp, x: i64, xt: IntTy, y: i64, yt: IntTy) -> R<i64> {
    match int_binary(op, x as i128, xt, y as i128, yt)? {
        Value::Int(v, _) => Ok(v as i64),
        other => Err(semantic(format!("integer operator produced {}", other.type_name()))),
    }
}

/// Result width of `l op r` on typed integers, or `None` when the operator is not an integer one.
pub(super) fn result_ty(op: BinaryOp, lt: IntTy, rt: IntTy) -> Option<IntTy> {
    use BinaryOp::*;
    match op {
        Add | Sub | Mul | Div | Mod | BitAnd | BitOr | BitXor => Some(IntTy::unify(lt, rt)),
        Shl | Shr => Some(lt),
        _ => None,
    }
}

macro_rules! checked {
    ($l:ident, $r:ident, $lt:ident, $rt:ident, $op:ident, $loc:ident, $range:ident, $f:expr) => {
        Box::new(move |it, b| {
            let x = $l.get(it, b)?;
            let y = $r.get(it, b)?;
            it.mark($loc);
            match $f(x, y) {
                Some(v) if $range.map_or(true, |(lo, hi)| v >= lo && v <= hi) => Ok(v),
                _ => slow($op, x, $lt, y, $rt),
            }
        })
    };
}

/// `l op r` on typed integers (operands left to right, then the operator at `loc`, as the walker).
pub(super) fn arith(op: BinaryOp, l: I, lt: IntTy, r: I, rt: IntTy, loc: Loc) -> Option<(ICode, IntTy)> {
    use BinaryOp::*;
    let ty = result_ty(op, lt, rt)?;
    let range = bounds(ty);
    let whole: Option<(i64, i64)> = None;
    let code: ICode = match op {
        Add => checked!(l, r, lt, rt, op, loc, range, i64::checked_add),
        Sub => checked!(l, r, lt, rt, op, loc, range, i64::checked_sub),
        Mul => checked!(l, r, lt, rt, op, loc, range, i64::checked_mul),
        Div => checked!(l, r, lt, rt, op, loc, range, |x: i64, y: i64| if y == 0 { None } else { x.checked_div(y) }),
        Mod => checked!(l, r, lt, rt, op, loc, range, |x: i64, y: i64| if y == 0 { None } else { x.checked_rem(y) }),
        BitAnd => checked!(l, r, lt, rt, op, loc, whole, |x: i64, y: i64| Some(x & y)),
        BitOr => checked!(l, r, lt, rt, op, loc, whole, |x: i64, y: i64| Some(x | y)),
        BitXor => checked!(l, r, lt, rt, op, loc, whole, |x: i64, y: i64| Some(x ^ y)),
        _ => Box::new(move |it, b| {
            let x = l.get(it, b)?;
            let y = r.get(it, b)?;
            it.mark(loc);
            slow(op, x, lt, y, rt)
        }),
    };
    Some((code, ty))
}

/// `l op r` for a comparison of typed integers (never fails once the operands are evaluated).
pub(super) fn compare(op: BinaryOp, l: I, r: I, loc: Loc) -> Option<super::fast::Cond> {
    use BinaryOp::*;
    macro_rules! cmp {
        ($f:expr) => {
            Box::new(move |it: &mut Interpreter, b: usize| {
                let x = l.get(it, b)?;
                let y = r.get(it, b)?;
                it.mark(loc);
                Ok($f(&x, &y))
            })
        };
    }
    Some(match op {
        Eq => cmp!(i64::eq),
        Ne => cmp!(i64::ne),
        Lt => cmp!(i64::lt),
        Le => cmp!(i64::le),
        Gt => cmp!(i64::gt),
        Ge => cmp!(i64::ge),
        _ => return None,
    })
}

/// `-x` on a typed integer: the walker's `negate` (`fits(-x)` or the overflow panic).
pub(super) fn negate(x: I, t: IntTy, loc: Loc) -> ICode {
    let range = bounds(t);
    Box::new(move |it, b| {
        let v = x.get(it, b)?;
        it.mark(loc);
        match v.checked_neg() {
            Some(n) if range.map_or(true, |(lo, hi)| n >= lo && n <= hi) => Ok(n),
            _ => Err(panic_ctl("attempt to negate with overflow")),
        }
    })
}
