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

//! FAILING REPRO — loop reuse of read-only `string` / `Vec` args must borrow.
//!
//! Ecosystem `wj-glob` `filter(pattern, paths)`:
//! ```ignore
//! while i < paths.len() {
//!     if is_match(pattern, paths[i]) { out.push(paths[i]) }
//! }
//! ```
//! Codegen moves `pattern` into `is_match` on the first iteration (E0382).
//! Read-only parameters must be borrowed so callers can reuse them in loops.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn loop_reuses_readonly_string_param_via_borrow() {
    let generated = test_utils::compile_single(
        r#"
pub fn is_match(pattern: string, path: string) -> bool {
    pattern == path
}

pub fn filter(pattern: string, paths: Vec<string>) -> Vec<string> {
    let mut out = Vec::new()
    let mut i = 0
    while i < paths.len() {
        if is_match(pattern, paths[i]) {
            out.push(paths[i])
        }
        i = i + 1
    }
    out
}
"#,
    );

    assert!(
        generated.contains("is_match(&pattern")
            || generated.contains("is_match(pattern.as_str()")
            || generated.contains("is_match(&*pattern"),
        "filter loop must borrow pattern for is_match:\n{generated}"
    );
}
