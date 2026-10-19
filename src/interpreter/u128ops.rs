//! `u128` arithmetic over the full range.
//!
//! Integers are stored as `i128`; a `u128` above `i128::MAX` keeps its bit pattern (a negative
//! `i128`), so every place that interprets the number goes through these helpers: arithmetic,
//! comparison, formatting, parsing, float conversion and the integer methods.

use super::error::{panic_ctl, R};
use super::int::IntTy;
use super::value::Value;
use crate::parser::BinaryOp;

#[inline]
pub(crate) fn bits(v: i128) -> u128 {
    v as u128
}

#[inline]
fn val(v: u128) -> Value {
    Value::Int(v as i128, IntTy::U128)
}

fn overflow(op: &str) -> super::error::Ctl {
    panic_ctl(format!("attempt to {} with overflow", op))
}

/// `a op b` for two `u128` operands (Rust's checked semantics: overflow panics).
pub(crate) fn binary(op: BinaryOp, a: i128, b: i128) -> R {
    let (x, y) = (bits(a), bits(b));
    Ok(match op {
        BinaryOp::Add => val(x.checked_add(y).ok_or_else(|| overflow("add"))?),
        BinaryOp::Sub => val(x.checked_sub(y).ok_or_else(|| overflow("subtract"))?),
        BinaryOp::Mul => val(x.checked_mul(y).ok_or_else(|| overflow("multiply"))?),
        BinaryOp::Div => {
            if y == 0 {
                return Err(panic_ctl("attempt to divide by zero"));
            }
            val(x / y)
        }
        BinaryOp::Mod => {
            if y == 0 {
                return Err(panic_ctl("attempt to calculate the remainder with a divisor of zero"));
            }
            val(x % y)
        }
        BinaryOp::BitAnd => val(x & y),
        BinaryOp::BitOr => val(x | y),
        BinaryOp::BitXor => val(x ^ y),
        BinaryOp::Shl | BinaryOp::Shr => {
            if b < 0 || b >= 128 {
                return Err(overflow(if op == BinaryOp::Shl { "shift left" } else { "shift right" }));
            }
            val(if op == BinaryOp::Shl { x << b } else { x >> b })
        }
        BinaryOp::Eq => Value::Bool(x == y),
        BinaryOp::Ne => Value::Bool(x != y),
        BinaryOp::Lt => Value::Bool(x < y),
        BinaryOp::Le => Value::Bool(x <= y),
        BinaryOp::Gt => Value::Bool(x > y),
        BinaryOp::Ge => Value::Bool(x >= y),
        BinaryOp::And | BinaryOp::Or => return Err(super::error::unsupported("logical operator on integers")),
    })
}

fn arg_bits(args: &[Value], i: usize) -> Option<u128> {
    match args.get(i) {
        Some(Value::Int(n, _)) => Some(bits(*n)),
        Some(Value::Ref(p)) => match p.read().ok()? {
            Value::Int(n, _) => Some(bits(n)),
            _ => None,
        },
        _ => None,
    }
}

/// Integer methods on a `u128` receiver; `None` for methods not specific to the width.
pub(crate) fn method(name: &str, a: i128, args: &[Value]) -> Option<R> {
    let x = bits(a);
    let y = || arg_bits(args, 0);
    let opt = |r: Option<u128>| Value::option(r.map(val));
    Some(Ok(match name {
        "wrapping_add" => val(x.wrapping_add(y()?)),
        "wrapping_sub" => val(x.wrapping_sub(y()?)),
        "wrapping_mul" => val(x.wrapping_mul(y()?)),
        "wrapping_pow" => val(x.wrapping_pow(y()? as u32)),
        "checked_add" => opt(x.checked_add(y()?)),
        "checked_sub" => opt(x.checked_sub(y()?)),
        "checked_mul" => opt(x.checked_mul(y()?)),
        "checked_div" => opt(x.checked_div(y()?)),
        "checked_rem" => opt(x.checked_rem(y()?)),
        "checked_pow" => opt(x.checked_pow(y()? as u32)),
        "saturating_add" => val(x.saturating_add(y()?)),
        "saturating_sub" => val(x.saturating_sub(y()?)),
        "saturating_mul" => val(x.saturating_mul(y()?)),
        "overflowing_add" => {
            let (r, o) = x.overflowing_add(y()?);
            Value::tuple(vec![val(r), Value::Bool(o)])
        }
        "overflowing_sub" => {
            let (r, o) = x.overflowing_sub(y()?);
            Value::tuple(vec![val(r), Value::Bool(o)])
        }
        "overflowing_mul" => {
            let (r, o) = x.overflowing_mul(y()?);
            Value::tuple(vec![val(r), Value::Bool(o)])
        }
        "pow" => match x.checked_pow(y()? as u32) {
            Some(r) => val(r),
            None => return Some(Err(overflow("multiply"))),
        },
        "count_ones" => Value::Int(x.count_ones() as i128, IntTy::U32),
        "count_zeros" => Value::Int(x.count_zeros() as i128, IntTy::U32),
        "leading_zeros" => Value::Int(x.leading_zeros() as i128, IntTy::U32),
        "trailing_zeros" => Value::Int(x.trailing_zeros() as i128, IntTy::U32),
        "is_power_of_two" => Value::Bool(x.is_power_of_two()),
        "to_string" => Value::str(x.to_string()),
        "max" => val(x.max(y()?)),
        "min" => val(x.min(y()?)),
        "abs_diff" => val(x.abs_diff(y()?)),
        _ => return None,
    }))
}

/// `u128` text for Display/Debug.
pub(crate) fn display(a: i128) -> String {
    bits(a).to_string()
}
