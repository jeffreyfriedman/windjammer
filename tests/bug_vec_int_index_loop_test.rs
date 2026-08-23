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

//! FAILING REPRO — `Vec` indexed by Windjammer `int` loop variable must cast to `usize`.
//!
//! Ecosystem `wj-yaml` first draft used `while idx < lines.len() { lines[idx] }` and hit
//! E0277: `[String]` cannot be indexed by `i64`. Workaround: for-in / head-tail without int indices.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn vec_int_index_must_cast_to_usize() {
    let generated = test_utils::compile_single(
        r#"
pub fn second(lines: Vec<string>) -> string {
    let mut idx = 0
    while idx < lines.len() {
        if idx == 1 {
            return lines[idx]
        }
        idx = idx + 1
    }
    ""
}
"#,
    );

    let has_cast = generated.contains("as usize")
        || generated.contains("try_into()")
        || generated.contains("usize::try_from");
    assert!(
        has_cast,
        "Vec index with int loop var must cast to usize:\n{generated}"
    );
}
