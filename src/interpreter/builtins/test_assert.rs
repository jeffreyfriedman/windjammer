//! `std::test` assertions that inspect Option/Result values and strings, mirroring the compiled
//! `wj test` surface (`windjammer_runtime::test`). Equality/ordering assertions live in `std_fns`.

use super::*;
use crate::interpreter::error::panic_ctl;

fn text(a: &[Value], i: usize, name: &str) -> R<String> {
    match arg(a, i, name)? {
        Value::Str(s) => Ok((*s).clone()),
        other => Err(panic_ctl(format!("{name}: expected a string argument, got {}", other.type_name()))),
    }
}

fn check(ok: bool, what: String) -> R {
    if ok { Ok(Value::Unit) } else { Err(panic_ctl(what)) }
}

fn variant_is(v: &Value, ty: &str, idx: u32) -> bool {
    matches!(v, Value::Enum(e) if &*e.ty == ty && e.idx == idx)
}

pub fn register(r: &mut Registry) {
    // `assert_contains(haystack, needle)`: substring for strings, membership for collections.
    r.func("assert_contains", |it, a| {
        let hay = arg(&a, 0, "assert_contains")?;
        let needle = arg(&a, 1, "assert_contains")?;
        if let (Value::Str(h), Value::Str(n)) = (&hay, &needle) {
            return check(h.contains(n.as_str()), format!("assert_contains failed: {:?} does not contain {:?}", h, n));
        }
        let items = it.to_items(hay)?;
        check(items.iter().any(|y| y.structural_eq(&needle)), "assert_contains failed: item not found".into())
    });
    r.func("assert_str_contains", |_, a| {
        let (h, n) = (text(&a, 0, "assert_str_contains")?, text(&a, 1, "assert_str_contains")?);
        check(h.contains(&n), format!("assert_str_contains failed: {:?} does not contain {:?}", h, n))
    });
    r.func("assert_starts_with", |_, a| {
        let (h, n) = (text(&a, 0, "assert_starts_with")?, text(&a, 1, "assert_starts_with")?);
        check(h.starts_with(&n), format!("assert_starts_with failed: {:?} does not start with {:?}", h, n))
    });
    r.func("assert_ends_with", |_, a| {
        let (h, n) = (text(&a, 0, "assert_ends_with")?, text(&a, 1, "assert_ends_with")?);
        check(h.ends_with(&n), format!("assert_ends_with failed: {:?} does not end with {:?}", h, n))
    });
    fn variant_assert(it: &mut Interpreter, a: &[Value], name: &str, ty: &str, idx: u32) -> R {
        let v = arg(a, 0, name)?;
        if variant_is(&v, ty, idx) {
            return Ok(Value::Unit);
        }
        let shown = it.debug_string(&v)?;
        Err(panic_ctl(format!("{name} failed: got {shown}")))
    }
    r.func("assert_is_ok", |it, a| variant_assert(it, &a, "assert_is_ok", "Result", 0));
    r.func("assert_result_ok", |it, a| variant_assert(it, &a, "assert_result_ok", "Result", 0));
    r.func("assert_is_err", |it, a| variant_assert(it, &a, "assert_is_err", "Result", 1));
    r.func("assert_result_err", |it, a| variant_assert(it, &a, "assert_result_err", "Result", 1));
    r.func("assert_is_some", |it, a| variant_assert(it, &a, "assert_is_some", "Option", 1));
    r.func("assert_is_none", |it, a| variant_assert(it, &a, "assert_is_none", "Option", 0));
    r.func("assert_length", |it, a| {
        let items = it.to_items(arg(&a, 0, "assert_length")?)?;
        let want = arg(&a, 1, "assert_length")?;
        let ok = matches!(want, Value::Int(n, _) if n as usize == items.len());
        check(ok, format!("assert_length failed: got length {}", items.len()))
    });
}
