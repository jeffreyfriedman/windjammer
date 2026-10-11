#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! `u64` compare must not emit `0_usize` literal.
//!
//! Product residual (~3× u64←usize), tip-out/gen sysbench:
//!   `if median == 0_usize` while `median: u64` → E0308 + E0277.
//! Zero literal must match u64 (`0` / `0_u64`).

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;

use tempfile::TempDir;
use windjammer::build_project;
use windjammer::CompilationTarget;

/// Tuple field `pair.0` from `(u64, bool)` compared to `0` must suffix `u64`.
/// `samples.len() == 0` may stay `usize`.
#[test]
fn tuple_u64_field_compare_zero_must_not_emit_usize() {
    let source = r#"
pub fn quiet(samples: Vec<u64>) -> (u64, bool) {
    if samples.len() == 0 {
        return (0, false)
    }
    (samples[0], false)
}

pub fn verdict(samples: Vec<u64>) -> bool {
    let pair = quiet(samples)
    let median = pair.0
    if median == 0 {
        return false
    }
    true
}
"#;
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("test.wj");
    fs::write(&wj, source).unwrap();
    let out = tmp.path().join("build");
    build_project(&wj, &out, CompilationTarget::Rust, false).expect("transpile");
    let rust = fs::read_to_string(out.join("test.rs")).expect("test.rs");
    eprintln!("isolate:\n{rust}");
    let median_cmp = rust.lines().find(|l| l.contains("median")).unwrap_or("");
    assert!(
        !median_cmp.contains("0_usize"),
        "u64 tuple field compared to 0_usize:\n{rust}"
    );
    test_utils::cargo_check_generated(&out);
}
