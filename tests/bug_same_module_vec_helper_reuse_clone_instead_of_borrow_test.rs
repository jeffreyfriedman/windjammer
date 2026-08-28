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

//! FAILING REPRO — same-module helper with reused `Vec<string>` must borrow, not `.clone()`.
//!
//! Ecosystem `wj-cors` `allow_origin` calls `is_origin_allowed(origin, allowed)` then reuses
//! `allowed` in a second loop. Codegen demotes both formals to `&Vec<String>` but emits:
//! ```ignore
//! if !is_origin_allowed(origin.clone(), allowed.clone()) {
//! ```
//! rustc E0308: expected `&Vec<String>`, found `Vec<String>`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn same_module_vec_helper_reuse_must_borrow_not_clone() {
    let generated = test_utils::compile_single(
        r#"
pub fn is_origin_allowed(origin: string, allowed: Vec<string>) -> bool {
    let mut i = 0
    while i < allowed.len() {
        if allowed[i] == origin {
            return true
        }
        i = i + 1
    }
    false
}

pub fn allow_origin(origin: string, allowed: Vec<string>) -> Option<string> {
    if !is_origin_allowed(origin, allowed) {
        return None
    }
    let mut j = 0
    while j < allowed.len() {
        if allowed[j] == "*" {
            return Some("*")
        }
        j = j + 1
    }
    Some(origin)
}
"#,
    );

    assert!(
        !generated.contains("is_origin_allowed(origin.clone(), allowed.clone())"),
        "reused Vec param must borrow at helper call site, not clone into owned Vec:\n{generated}"
    );
    assert!(
        generated.contains("is_origin_allowed(") && !generated.contains("allowed.clone())"),
        "expected borrow-style helper call without allowed.clone():\n{generated}"
    );
}
