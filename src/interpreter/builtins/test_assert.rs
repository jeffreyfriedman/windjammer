//! `std::test`: the one assertion module, mirroring the compiled surface (`windjammer_runtime::test`).
//!
//! `assert`, `assert_eq` and `assert_ne` are intrinsics (`interpreter/intrinsics.rs`, plus the `test::` aliases in
//! `std_legacy`). Every function here panics with the same text as its runtime twin in
//! `crates/windjammer-runtime/src/test.rs`; `tests/std_test_module_test.rs` runs both and compares the messages.

use super::*;
use crate::interpreter::error::panic_ctl;
use std::cmp::Ordering as O;

fn fail(text: String) -> R {
    Err(panic_ctl(text))
}

fn text(a: &[Value], i: usize, name: &str) -> R<String> {
    match arg(a, i, name)? {
        Value::Str(s) => Ok((*s).clone()),
        other => Err(panic_ctl(format!("{name}: expected a string argument, got {}", other.type_name()))),
    }
}

fn is_variant(v: &Value, ty: &str, idx: u32) -> bool {
    matches!(v, Value::Enum(e) if &*e.ty == ty && e.idx == idx)
}

/// The payload of a one-field `Option` / `Result` variant.
fn payload(v: &Value) -> Option<Value> {
    match v {
        Value::Enum(e) => match &e.payload {
            Payload::Tuple(p) => p.first().cloned(),
            _ => None,
        },
        _ => None,
    }
}

fn compare(it: &mut Interpreter, a: &[Value], name: &str, op: &str, ok: fn(O) -> bool) -> R {
    let (x, y) = (arg(a, 0, name)?, arg(a, 1, name)?);
    if ok(x.structural_cmp(&y)) {
        return Ok(Value::Unit);
    }
    let (l, r) = (it.debug_string(&x)?, it.debug_string(&y)?);
    fail(format!("assertion failed: left {op} right\n  left: {l}\n right: {r}"))
}

/// Both widths: an `f32` operand makes the whole comparison `f32`, as the compiled generic does.
fn approx_eq(it: &mut Interpreter, a: &[Value]) -> R {
    let name = "assert_approx_eq";
    let vals = [arg(a, 0, name)?, arg(a, 1, name)?, arg(a, 2, name)?];
    let narrow = vals.iter().any(|v| matches!(v, Value::Float(_, FloatTy::F32)));
    let [l, r, e] = [arg_f64(a, 0, name)?, arg_f64(a, 1, name)?, arg_f64(a, 2, name)?];
    let (ty, diff) = if narrow {
        (FloatTy::F32, ((l as f32) - (r as f32)).abs() as f64)
    } else {
        (FloatTy::F64, (l - r).abs())
    };
    let within = if narrow { (diff as f32) <= (e as f32) } else { diff <= e };
    if within {
        return Ok(Value::Unit);
    }
    let mut show = |x: f64| it.display_string(&Value::Float(x, ty));
    let (sl, sr, se, sd) = (show(l)?, show(r)?, show(e)?, show(diff)?);
    fail(format!("assertion failed: values not approximately equal\n  left: {sl}\n  right: {sr}\n  epsilon: {se}\n  diff: {sd}"))
}

fn in_range(it: &mut Interpreter, a: &[Value]) -> R {
    let name = "assert_in_range";
    let (x, lo, hi) = (arg(a, 0, name)?, arg(a, 1, name)?, arg(a, 2, name)?);
    if x.structural_cmp(&lo) != O::Less && x.structural_cmp(&hi) != O::Greater {
        return Ok(Value::Unit);
    }
    let (sx, slo, shi) = (it.debug_string(&x)?, it.debug_string(&lo)?, it.debug_string(&hi)?);
    fail(format!("assertion failed: value not in range\n  value: {sx}\n  min: {slo}\n  max: {shi}"))
}

fn some(it: &mut Interpreter, a: &[Value]) -> R {
    let v = arg(a, 0, "assert_some")?;
    let _ = it;
    if is_variant(&v, "Option", 1) {
        return Ok(payload(&v).unwrap_or(Value::Unit));
    }
    fail("assertion failed: expected Some, got None".into())
}

fn none(it: &mut Interpreter, a: &[Value]) -> R {
    let v = arg(a, 0, "assert_none")?;
    if is_variant(&v, "Option", 0) {
        return Ok(Value::Unit);
    }
    let shown = it.debug_string(&payload(&v).unwrap_or(Value::Unit))?;
    fail(format!("assertion failed: expected None, got Some({shown})"))
}

fn ok(it: &mut Interpreter, a: &[Value]) -> R {
    let v = arg(a, 0, "assert_ok")?;
    if is_variant(&v, "Result", 0) {
        return Ok(payload(&v).unwrap_or(Value::Unit));
    }
    let shown = it.debug_string(&payload(&v).unwrap_or(Value::Unit))?;
    fail(format!("assertion failed: expected Ok, got Err({shown})"))
}

fn err(it: &mut Interpreter, a: &[Value]) -> R {
    let v = arg(a, 0, "assert_err")?;
    if is_variant(&v, "Result", 1) {
        return Ok(payload(&v).unwrap_or(Value::Unit));
    }
    let shown = it.debug_string(&payload(&v).unwrap_or(Value::Unit))?;
    fail(format!("assertion failed: expected Err, got Ok({shown})"))
}

/// Substring for strings, membership for collections.
fn contains(it: &mut Interpreter, a: &[Value]) -> R {
    let hay = arg(a, 0, "assert_contains")?;
    let needle = arg(a, 1, "assert_contains")?;
    if let (Value::Str(h), Value::Str(n)) = (&hay, &needle) {
        if h.contains(n.as_str()) {
            return Ok(Value::Unit);
        }
        return fail(format!("assertion failed: string does not contain substring\n  string: {:?}\n  substring: {:?}", h, n));
    }
    let items = it.to_items(hay.clone())?;
    if items.iter().any(|y| y.structural_eq(&needle)) {
        return Ok(Value::Unit);
    }
    let (shown, item) = (it.debug_string(&hay)?, it.debug_string(&needle)?);
    fail(format!("assertion failed: collection does not contain item\n  collection: {shown}\n  item: {item}"))
}

fn starts_with(_: &mut Interpreter, a: &[Value]) -> R {
    let (s, p) = (text(a, 0, "assert_starts_with")?, text(a, 1, "assert_starts_with")?);
    if s.starts_with(&p) {
        return Ok(Value::Unit);
    }
    fail(format!("assertion failed: string does not start with prefix\n  string: {:?}\n  prefix: {:?}", s, p))
}

fn ends_with(_: &mut Interpreter, a: &[Value]) -> R {
    let (s, p) = (text(a, 0, "assert_ends_with")?, text(a, 1, "assert_ends_with")?);
    if s.ends_with(&p) {
        return Ok(Value::Unit);
    }
    fail(format!("assertion failed: string does not end with suffix\n  string: {:?}\n  suffix: {:?}", s, p))
}

/// Strings count bytes (as `str::len` does when compiled); everything else counts its items.
fn length_of(it: &mut Interpreter, v: Value) -> R<usize> {
    match v {
        Value::Str(s) => Ok(s.len()),
        other => Ok(it.to_items(other)?.len()),
    }
}

fn empty(it: &mut Interpreter, a: &[Value]) -> R {
    let n = length_of(it, arg(a, 0, "assert_empty")?)?;
    if n == 0 {
        return Ok(Value::Unit);
    }
    fail(format!("assertion failed: expected empty, got length {n}"))
}

fn not_empty(it: &mut Interpreter, a: &[Value]) -> R {
    if length_of(it, arg(a, 0, "assert_not_empty")?)? != 0 {
        return Ok(Value::Unit);
    }
    fail("assertion failed: expected non-empty, got length 0".into())
}

fn length(it: &mut Interpreter, a: &[Value]) -> R {
    let actual = length_of(it, arg(a, 0, "assert_length")?)?;
    let expected = arg_int(a, 1, "assert_length")?;
    if actual as i128 == expected {
        return Ok(Value::Unit);
    }
    fail(format!("assertion failed: length mismatch\n  expected: {expected}\n  actual: {actual}"))
}

/// `fail(message)`: the runtime's `Test failed: {message}`.
fn fail_test(it: &mut Interpreter, a: &[Value]) -> R {
    let message = it.display_string(&arg(a, 0, "fail")?)?;
    fail(format!("Test failed: {message}"))
}

pub fn register(r: &mut Registry) {
    r.func("test::fail", |it, a| fail_test(it, &a));
    r.func("fail", |it, a| fail_test(it, &a));
    r.func("assert_gt", |it, a| compare(it, &a, "assert_gt", ">", |o| o == O::Greater));
    r.func("assert_lt", |it, a| compare(it, &a, "assert_lt", "<", |o| o == O::Less));
    r.func("assert_ge", |it, a| compare(it, &a, "assert_ge", ">=", |o| o != O::Less));
    r.func("assert_le", |it, a| compare(it, &a, "assert_le", "<=", |o| o != O::Greater));
    r.func("assert_approx_eq", |it, a| approx_eq(it, &a));
    r.func("assert_in_range", |it, a| in_range(it, &a));
    r.func("assert_some", |it, a| some(it, &a));
    r.func("assert_none", |it, a| none(it, &a));
    r.func("assert_ok", |it, a| ok(it, &a));
    r.func("assert_err", |it, a| err(it, &a));
    r.func("assert_contains", |it, a| contains(it, &a));
    r.func("assert_starts_with", |it, a| starts_with(it, &a));
    r.func("assert_ends_with", |it, a| ends_with(it, &a));
    r.func("assert_empty", |it, a| empty(it, &a));
    r.func("assert_not_empty", |it, a| not_empty(it, &a));
    r.func("assert_length", |it, a| length(it, &a));
}
