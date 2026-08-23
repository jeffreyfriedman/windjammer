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

//! FAILING REPRO — `vec.len() - int` in a loop condition must unify as int or cast both sides.
//!
//! Ecosystem `wj-migrate` bubble sort used `while j + 1 < n - i` where `n = out.len()` (usize)
//! and `i` is Windjammer `int` (i64). Codegen emitted `((n - i as usize as i64))` → E0277/E0308.
//! Workaround: insertion sort without `len() - int`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn vec_len_minus_int_loop_bound_must_unify() {
    let generated = test_utils::compile_single(
        r#"
pub fn bubble(count: int) -> int {
    let mut n = count
    // Simulate: n from a Vec.len() style usize value compared with int arithmetic.
    let mut i = 0
    let mut steps = 0
    while i < n {
        let mut j = 0
        while j + 1 < n - i {
            steps = steps + 1
            j = j + 1
        }
        i = i + 1
    }
    steps
}
"#,
    );

    // Prefer int-side arithmetic: cast len once to i64, not `usize - i64`.
    assert!(
        !generated.contains("n - i as usize as i64")
            && !generated.contains("as usize as i64"),
        "len()-int subtraction must not emit usize - i64:\n{generated}"
    );
}
