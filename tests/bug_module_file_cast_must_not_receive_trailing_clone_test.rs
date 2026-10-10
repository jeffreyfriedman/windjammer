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
    feature = "codegen_tests",
))]

//! WDB-300: cast expr must not grow a trailing `.clone()` (`n as usize.clone()`).
//!
//! Product tip-out/gen PageRank / pg_serve:
//!   `graph_pagerank_buffers_new(n as usize.clone(), …)` → "cast cannot be followed by a method call".
//! WJ source: `graph_pagerank_buffers_new(n, 1.0 / n as f64)` — clone coercion must not attach after `as`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn buffers_new(n: usize, init: f64) -> f64 {
    init + (n as f64)
}

pub fn buffers_seeded(n: usize, prior_len: usize) -> f64 {
    if prior_len != n {
        return buffers_new(n, 1.0 / n as f64)
    }
    0.0
}
"#;

#[test]
fn module_file_cast_must_not_emit_trailing_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as usize.clone()")
        || rs.contains("as u32.clone()")
        || rs.contains("as i64.clone()")
        || rs.contains("as f64.clone()");
    assert!(
        !bad,
        "MultiFile emitted cast-then-.clone() (illegal Rust):\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
