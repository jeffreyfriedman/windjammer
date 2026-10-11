//! Integer/float representation rules for JavaScript.
//!
//! 64-bit integers are BigInt (exact); narrower integers are Numbers kept in range by checked
//! arithmetic (panic on overflow, matching Rust debug semantics) or explicit wrapping helpers.

use crate::codegen::portable::ir::BinKind;
use crate::codegen::portable::ty::{IntKind, Ty};

pub fn is_wide(k: IntKind) -> bool {
    k.is_wide()
}

pub fn lit_int(v: i128, k: IntKind) -> String {
    let neg = v < 0;
    let body = if is_wide(k) { format!("{}n", v.unsigned_abs()) } else { v.unsigned_abs().to_string() };
    if neg { format!("(-{body})") } else { body }
}

/// Name of the range-check helper for a kind (`$i32`, `$u64`, ...).
pub fn chk(k: IntKind) -> &'static str {
    match k {
        IntKind::I8 => "$i8",
        IntKind::I16 => "$i16",
        IntKind::I32 => "$i32",
        IntKind::I64 | IntKind::Isize => "$i64",
        IntKind::U8 => "$u8",
        IntKind::U16 => "$u16",
        IntKind::U32 => "$u32",
        IntKind::U64 | IntKind::Usize => "$u64",
    }
}

fn bits_signed(k: IntKind) -> (u32, bool) {
    (k.bits(), k.signed())
}

/// Wrap an arbitrary-precision/Number expression into the kind's range (two's complement).
pub fn wrap_expr(k: IntKind, e: &str) -> String {
    let (bits, signed) = bits_signed(k);
    if is_wide(k) {
        return format!("BigInt.{}({bits}, {e})", if signed { "asIntN" } else { "asUintN" });
    }
    match (bits, signed) {
        (32, true) => format!("(({e}) | 0)"),
        (32, false) => format!("(({e}) >>> 0)"),
        (16, true) => format!("((({e}) << 16) >> 16)"),
        (16, false) => format!("(({e}) & 65535)"),
        (8, true) => format!("((({e}) << 24) >> 24)"),
        _ => format!("(({e}) & 255)"),
    }
}

fn op_str(op: BinKind) -> &'static str {
    match op {
        BinKind::Add => "+",
        BinKind::Sub => "-",
        BinKind::Mul => "*",
        BinKind::BitAnd => "&",
        BinKind::BitOr => "|",
        BinKind::BitXor => "^",
        BinKind::Lt => "<",
        BinKind::Le => "<=",
        BinKind::Gt => ">",
        BinKind::Ge => ">=",
        _ => "?",
    }
}

/// Integer binary operation with Rust-debug semantics (overflow and division by zero panic).
pub fn int_binary(op: BinKind, k: IntKind, a: &str, b: &str) -> String {
    let wide = is_wide(k);
    match op {
        BinKind::Add | BinKind::Sub | BinKind::Mul => format!("{}({a} {} {b})", chk(k), op_str(op)),
        BinKind::Div => format!("{}({}({a}, {b}))", chk(k), if wide { "$divBig" } else { "$divNum" }),
        BinKind::Rem => format!("{}({}({a}, {b}))", chk(k), if wide { "$remBig" } else { "$remNum" }),
        BinKind::BitAnd | BinKind::BitOr | BinKind::BitXor => {
            if k.signed() || wide || k.bits() < 32 {
                format!("({a} {} {b})", op_str(op))
            } else {
                format!("(({a} {} {b}) >>> 0)", op_str(op))
            }
        }
        BinKind::Eq => format!("({a} === {b})"),
        BinKind::Ne => format!("({a} !== {b})"),
        BinKind::Lt | BinKind::Le | BinKind::Gt | BinKind::Ge => format!("({a} {} {b})", op_str(op)),
        BinKind::Shl | BinKind::Shr | BinKind::And | BinKind::Or => unreachable!("handled by caller"),
    }
}

/// Shift with the count coerced to the operand representation; count >= bits panics.
pub fn shift(left: bool, k: IntKind, a: &str, count: &str, count_wide: bool) -> String {
    let bits = k.bits();
    let n = if count_wide { format!("Number({count})") } else { count.to_string() };
    let guard = format!("$shc({n}, {bits})");
    let (c, signed) = (if is_wide(k) { format!("BigInt({guard})") } else { guard }, k.signed());
    if left {
        wrap_expr(k, &format!("{a} << {c}"))
    } else if !is_wide(k) && !signed && bits == 32 {
        format!("({a} >>> {c})")
    } else {
        format!("({a} >> {c})")
    }
}

pub fn float_lit(f: f64, is32: bool) -> String {
    let s = if f.is_nan() {
        "NaN".to_string()
    } else if f.is_infinite() {
        if f > 0.0 { "Infinity".into() } else { "(-Infinity)".into() }
    } else if f == f.trunc() && f.abs() < 1e15 {
        format!("{f:.1}")
    } else {
        format!("{f:?}")
    };
    if is32 { format!("Math.fround({s})") } else if f < 0.0 && !s.starts_with('(') { format!("({s})") } else { s }
}

/// `e as to` for numeric/char/bool sources.
pub fn cast(from: &Ty, to: &Ty, e: &str) -> String {
    match (from, to) {
        (Ty::Int(a), Ty::Int(b)) => int_to_int(*a, *b, e),
        (Ty::Int(a), Ty::F64) => if is_wide(*a) { format!("Number({e})") } else { e.to_string() },
        (Ty::Int(a), Ty::F32) => format!("Math.fround({})", if is_wide(*a) { format!("Number({e})") } else { e.to_string() }),
        (Ty::F64 | Ty::F32, Ty::Int(b)) => format!("$f2i({e}, {}, {}, {})", lo(*b), hi(*b), is_wide(*b)),
        (Ty::F64, Ty::F32) => format!("Math.fround({e})"),
        (Ty::F32, Ty::F64) | (Ty::F64, Ty::F64) | (Ty::F32, Ty::F32) => e.to_string(),
        (Ty::Bool, Ty::Int(b)) => if is_wide(*b) { format!("BigInt(+{e})") } else { format!("(+{e})") },
        (Ty::Char, Ty::Int(b)) => int_to_int(IntKind::U32, *b, &format!("{e}.codePointAt(0)")),
        (Ty::Int(IntKind::U8), Ty::Char) => format!("String.fromCodePoint({e})"),
        (Ty::Int(a), Ty::Char) => format!("String.fromCodePoint(Number({}))", if is_wide(*a) { e.to_string() } else { e.to_string() }),
        _ => e.to_string(),
    }
}

fn lo(k: IntKind) -> String {
    if k.signed() { format!("-(2n ** {}n)", k.bits() - 1) } else { "0n".into() }
}

fn hi(k: IntKind) -> String {
    if k.signed() { format!("(2n ** {}n) - 1n", k.bits() - 1) } else { format!("(2n ** {}n) - 1n", k.bits()) }
}

fn int_to_int(a: IntKind, b: IntKind, e: &str) -> String {
    match (is_wide(a), is_wide(b)) {
        (true, true) => if a.signed() == b.signed() { e.to_string() } else { wrap_expr(b, e) },
        (false, true) => wrap_expr(b, &format!("BigInt({e})")),
        (true, false) => format!("Number({})", wrap_big(b, e)),
        (false, false) => if a == b { e.to_string() } else { wrap_expr(b, e) },
    }
}

fn wrap_big(k: IntKind, e: &str) -> String {
    format!("BigInt.{}({}, {e})", if k.signed() { "asIntN" } else { "asUintN" }, k.bits())
}

/// Zero value literal of an int kind.
pub fn zero(k: IntKind) -> String {
    if is_wide(k) { "0n".into() } else { "0".into() }
}

pub fn one(k: IntKind) -> String {
    if is_wide(k) { "1n".into() } else { "1".into() }
}
