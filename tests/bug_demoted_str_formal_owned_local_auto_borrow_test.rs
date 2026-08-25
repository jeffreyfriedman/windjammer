//! Read-only helper demoted to `&str` must auto-borrow an owned local at the call site.
//!
//! Ecosystem `wj-multipart`:
//! ```
//! let chunk = strings.trim(chunks[i])
//! match parse_part(chunk) { … }
//! ```
//! Analyzer demotes `parse_part(chunk: string)` → `&str`, but codegen still
//! passes the owned `String` (E0308 expected `&str`, found `String`).

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn demoted_str_formal_must_auto_borrow_owned_local() {
    let source = r#"
use std::strings

fn looks_empty(chunk: string) -> bool {
    strings.len(chunk) == 0
}

pub fn any_empty(parts: Vec<string>) -> bool {
    let mut i = 0
    while i < parts.len() {
        let chunk = strings.trim(parts[i])
        if looks_empty(chunk) {
            return true
        }
        i = i + 1
    }
    false
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "owned local into demoted &str formal must auto-borrow, got:\n{generated}"
    );
}
