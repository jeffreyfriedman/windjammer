#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO — `AtomicI64` is not `Clone`. Structs that wrap it must not
//! auto-derive `Clone` (wj-sync bare-Counter hot path).

#[path = "common/test_utils.rs"]
mod test_utils;

fn last_derive_before(rust: &str, byte_idx: usize) -> Option<&str> {
    let prev = &rust[..byte_idx];
    let dpos = prev.rfind("#[derive(")?;
    let rest = &prev[dpos..];
    let end = rest.find("]\n").or_else(|| rest.find(']'))?;
    Some(&rest[..=end])
}

const COUNTER: &str = r#"
use std::sync::atomic::{AtomicI64, Ordering}

pub struct Counter {
    inner: AtomicI64,
}

pub fn counter_new(n: int) -> Counter {
    Counter {
        inner: AtomicI64::new(n),
    }
}

pub fn counter_inc(c: Counter) {
    c.inner.fetch_add(1, Ordering::Relaxed)
}

pub fn counter_get(c: Counter) -> int {
    c.inner.load(Ordering::Relaxed)
}
"#;

#[test]
fn atomic_i64_struct_must_not_derive_clone() {
    let output = test_utils::compile_single(COUNTER);
    assert!(!output.is_empty(), "expected generated Rust");
    let needle = "pub struct Counter";
    let idx = output
        .find(needle)
        .unwrap_or_else(|| panic!("missing {needle}\n{output}"));
    if let Some(attr) = last_derive_before(&output, idx) {
        assert!(
            !attr.contains("Clone"),
            "Counter with AtomicI64 must not #[derive(..., Clone, ...)]; found: {attr}\n\n{output}"
        );
    }
}

#[test]
fn atomic_i64_struct_cargo_check_succeeds() {
    test_utils::assert_stdlib_runtime_links(
        COUNTER,
        &["AtomicI64", "fetch_add", "Ordering"],
    );
}
