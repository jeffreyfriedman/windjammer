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

//! P3.755: untyped `let mut total = 0` accumulated with `g.len()` must be `usize`.
//!
//! `shader_graph_executor.wj` writes `total_async = total_async + g.len()` after
//! `let mut total_async = 0`. Tip-out emits `0_i64` and adds a `usize` length.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn untyped_zero_plus_vec_len_must_not_emit_i64() {
    let source = r#"
fn tally(groups: Vec<Vec<i32>>) {
    let mut total_async = 0
    for g in groups {
        total_async = total_async + g.len()
    }
}
"#;
    let rust = test_utils::compile_single(source);
    eprintln!("P3.755 tally:\n{rust}");
    assert!(
        !rust.contains("0_i64"),
        "P3.755 RED: untyped zero plus Vec::len() emitted 0_i64:\n{rust}"
    );
    assert!(
        rust.contains("0_usize"),
        "P3.755 RED: accumulator init must be 0_usize:\n{rust}"
    );
}
