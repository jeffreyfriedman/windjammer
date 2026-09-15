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

//! FAILING REPRO — thin-wrap `mime.from_path(path)` emits `path.clone()` on `impl Into<String>`.
//!
//! Ecosystem `wj-mime`:
//! ```ignore
//! pub fn from_path(path: string) -> string {
//!     mime.from_path(path)
//! }
//! ```
//! Tip codegen: `pub fn from_path(path: impl Into<String>)` + `mime::from_path(&path.clone())` → E0599.

#[path = "common/test_utils.rs"]
mod test_utils;

const WRAP: &str = r#"
use std::mime

pub fn from_path(path: string) -> string {
    mime.from_path(path)
}
"#;

#[test]
fn mime_from_path_thin_wrap_must_not_clone_into_string() {
    let generated = test_utils::compile_single(WRAP);
    assert!(
        !generated.contains("path.clone()")
            && !generated.contains("&path.clone()"),
        "thin-wrap mime.from_path must not clone Into<String> formal:\n{generated}"
    );
}

#[test]
fn mime_from_path_thin_wrap_must_cargo_check() {
    test_utils::assert_stdlib_runtime_links(WRAP, &["mime::from_path"]);
}
