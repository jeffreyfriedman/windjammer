//! Hot-shape specializations of the resolved engine: operands read from slots without cloning,
//! integer arithmetic in `i64` when both operands and the result fit (any edge -- overflow, a zero
//! divisor, `u64`/`i128`/`u128` widths -- falls back to `ops::int_binary`, so results and panic
//! messages are the walker's), and conditions that yield `bool` instead of a `Value::Bool`.

use super::Code;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::{semantic, R};
use crate::interpreter::int::IntTy;
use crate::interpreter::ops::int_binary;
use crate::interpreter::place::deref_value;
use crate::interpreter::value::Value;
use crate::parser::BinaryOp;
use crate::source_map::Location;

pub(super) type Cond = Box<dyn Fn(&mut Interpreter, usize) -> R<bool>>;
type Loc = Option<&'static Location>;

/// An operand: a local slot or a constant are read inline, anything else runs its code.
pub(super) enum Operand {
    Slot(usize),
    Const(Value),
    /// A typed integer expression (`typed.rs`).
    Int(super::typed::ICode, IntTy),
    Code(Code),
}

/// An evaluated operand: an integer by value, or any other value.
enum Got {
    Int(i128, IntTy),
    Val(Value),
}

impl Got {
    fn value(self) -> Value {
        match self {
            Got::Int(a, t) => Value::Int(a, t),
            Got::Val(v) => v,
        }
    }
}

#[inline(always)]
fn get(it: &mut Interpreter, b: usize, o: &Operand) -> R<Got> {
    let v = match o {
        Operand::Slot(s) => match &it.rstack[b + s] {
            Value::Int(a, t) => return Ok(Got::Int(*a, *t)),
            v => v.clone(),
        },
        Operand::Const(Value::Int(a, t)) => return Ok(Got::Int(*a, *t)),
        Operand::Const(v) => v.clone(),
        Operand::Int(c, t) => return Ok(Got::Int(c(it, b)? as i128, *t)),
        Operand::Code(c) => c(it, b)?,
    };
    Ok(match v {
        Value::Int(a, t) => Got::Int(a, t),
        v => Got::Val(v),
    })
}

fn is_cmp(op: BinaryOp) -> bool {
    matches!(op, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge)
}

/// `a op b` on integers; same results as `int_binary`, cheaper on the common path.
#[inline(always)]
pub(super) fn int_op(op: BinaryOp, a: i128, at: IntTy, b: i128, bt: IntTy) -> R {
    let ty = IntTy::unify(at, bt);
    if ty != IntTy::U128 && ty != IntTy::I128 && a as i64 as i128 == a && b as i64 as i128 == b {
        let (x, y) = (a as i64, b as i64);
        let r = match op {
            BinaryOp::Add => x.checked_add(y),
            BinaryOp::Sub => x.checked_sub(y),
            BinaryOp::Mul => x.checked_mul(y),
            BinaryOp::Div if y != 0 => x.checked_div(y),
            BinaryOp::Mod if y != 0 => x.checked_rem(y),
            _ => None,
        };
        if let Some(r) = r {
            if ty.fits(r as i128) {
                return Ok(Value::Int(r as i128, ty));
            }
        }
    }
    if let Some(c) = int_cmp(op, a, at, b, bt) {
        return Ok(Value::Bool(c));
    }
    int_binary(op, a, at, b, bt)
}

/// A comparison of two integers (`None`: not a comparison, or `u128` ordering applies).
#[inline(always)]
fn int_cmp(op: BinaryOp, a: i128, at: IntTy, b: i128, bt: IntTy) -> Option<bool> {
    if IntTy::unify(at, bt) == IntTy::U128 {
        return None;
    }
    Some(match op {
        BinaryOp::Eq => a == b,
        BinaryOp::Ne => a != b,
        BinaryOp::Lt => a < b,
        BinaryOp::Le => a <= b,
        BinaryOp::Gt => a > b,
        BinaryOp::Ge => a >= b,
        _ => return None,
    })
}

/// `l op r` (not `&&`/`||`): operands left to right, then the operator at `loc`.
pub(super) fn binary(l: Operand, r: Operand, op: BinaryOp, loc: Loc) -> Code {
    Box::new(move |it, b| {
        let x = get(it, b, &l)?;
        let y = get(it, b, &r)?;
        it.mark(loc);
        match (x, y) {
            (Got::Int(a, at), Got::Int(c, ct)) => int_op(op, a, at, c, ct),
            (x, y) => it.binary(op, x.value(), y.value()),
        }
    })
}

/// A condition from a comparison: integers compare without building a `Value`.
pub(super) fn compare(l: Operand, r: Operand, op: BinaryOp, loc: Loc, what: &'static str) -> Option<Cond> {
    if !is_cmp(op) {
        return None;
    }
    Some(Box::new(move |it, b| {
        let x = get(it, b, &l)?;
        let y = get(it, b, &r)?;
        it.mark(loc);
        if let (Got::Int(a, at), Got::Int(c, ct)) = (&x, &y) {
            if let Some(c) = int_cmp(op, *a, *at, *c, *ct) {
                return Ok(c);
            }
        }
        let v = match (x, y) {
            (Got::Int(a, at), Got::Int(c, ct)) => int_binary(op, a, at, c, ct)?,
            (x, y) => it.binary(op, x.value(), y.value())?,
        };
        truth(v, what)
    }))
}

/// Any other condition: its value must be a `bool` (`if` looks through references, as the walker).
pub(super) fn general(c: Code, what: &'static str, deref: bool) -> Cond {
    Box::new(move |it, b| {
        let v = c(it, b)?;
        truth(if deref { deref_value(v)? } else { v }, what)
    })
}

fn truth(v: Value, what: &str) -> R<bool> {
    match v {
        Value::Bool(x) => Ok(x),
        other => Err(semantic(format!("`{what}` condition must be bool, found {}", other.type_name()))),
    }
}

/// `rstack[i] = v`, skipping the destructor call when the old value owns nothing (the common case in loops).
#[inline(always)]
pub(super) fn set(it: &mut Interpreter, i: usize, v: Value) {
    let slot = &mut it.rstack[i];
    if matches!(slot, Value::Int(..) | Value::Bool(_) | Value::Unit | Value::Float(..) | Value::Char(_)) {
        std::mem::forget(std::mem::replace(slot, v));
    } else {
        *slot = v;
    }
}

/// `slot op= rhs` on an integer slot, updated in place; `None` = not two integers (caller's path).
#[inline(always)]
pub(super) fn compound_int(it: &mut Interpreter, b: usize, slot: usize, op: BinaryOp, rhs: &Value) -> Option<R<()>> {
    let (Value::Int(a, at), Value::Int(c, ct)) = (&it.rstack[b + slot], rhs) else { return None };
    Some(int_op(op, *a, *at, *c, *ct).map(|v| set(it, b + slot, v)))
}
