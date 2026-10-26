//! Numeric emission rules for Go.

use super::types::int_name;
use crate::codegen::portable::ir::BinKind;
use crate::codegen::portable::ty::{IntKind, Ty};

pub fn lit_int(v: i128, k: IntKind) -> String {
    format!("{}({v})", int_name(k))
}

pub fn float_lit(f: f64, is32: bool) -> String {
    let ty = if is32 { "float32" } else { "float64" };
    if f == 0.0 && f.is_sign_negative() {
        return format!("{ty}(wjNegZero())");
    }
    if f.is_nan() {
        return format!("{ty}(wjNaN())");
    }
    if f.is_infinite() {
        return format!("{ty}(wjInf({}))", if f > 0.0 { 1 } else { -1 });
    }
    if f == f.trunc() && f.abs() < 1e15 {
        format!("{ty}({f:.1})")
    } else {
        format!("{ty}({f:?})")
    }
}

pub fn int_binary(op: BinKind, a: &str, b: &str) -> String {
    match op {
        BinKind::Add => format!("wjAdd({a}, {b})"),
        BinKind::Sub => format!("wjSub({a}, {b})"),
        BinKind::Mul => format!("wjMul({a}, {b})"),
        BinKind::Div => format!("wjDiv({a}, {b})"),
        BinKind::Rem => format!("wjRem({a}, {b})"),
        BinKind::BitAnd => format!("({a} & {b})"),
        BinKind::BitOr => format!("({a} | {b})"),
        BinKind::BitXor => format!("({a} ^ {b})"),
        BinKind::Shl => format!("wjShl({a}, {b})"),
        BinKind::Shr => format!("wjShr({a}, {b})"),
        BinKind::Eq => format!("({a} == {b})"),
        BinKind::Ne => format!("({a} != {b})"),
        BinKind::Lt => format!("({a} < {b})"),
        BinKind::Le => format!("({a} <= {b})"),
        BinKind::Gt => format!("({a} > {b})"),
        BinKind::Ge => format!("({a} >= {b})"),
        BinKind::And | BinKind::Or => unreachable!(),
    }
}

/// `e as to` conversion.
pub fn cast(from: &Ty, to: &Ty, e: &str) -> String {
    match (from, to) {
        (Ty::Int(a), Ty::Int(b)) => if a == b { e.to_string() } else { format!("{}({e})", int_name(*b)) },
        (Ty::Int(_), Ty::F64) => format!("float64({e})"),
        (Ty::Int(_), Ty::F32) => format!("float32({e})"),
        (Ty::F64 | Ty::F32, Ty::Int(b)) => format!("wjF2I[{}](float64({e}))", int_name(*b)),
        (Ty::F64, Ty::F32) => format!("float32({e})"),
        (Ty::F32, Ty::F64) => format!("float64({e})"),
        (Ty::Bool, Ty::Int(b)) => format!("wjB2I[{}]({e})", int_name(*b)),
        (Ty::Char, Ty::Int(b)) => format!("{}({e})", int_name(*b)),
        (Ty::Int(_), Ty::Char) => format!("Char({e})"),
        // A generic value converted to a concrete type (`T: Into<string>`): checked assertion.
        (Ty::Param(_), Ty::Str) => format!("any({e}).(string)"),
        _ => e.to_string(),
    }
}

pub fn range_of(k: IntKind) -> (i128, i128) {
    let b = k.bits();
    if k.signed() { (-(1i128 << (b - 1)), (1i128 << (b - 1)) - 1) } else { (0, (1i128 << b) - 1) }
}
