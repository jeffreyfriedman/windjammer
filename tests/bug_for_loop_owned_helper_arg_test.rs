//! Borrowed `for item in vec` loop variables must `.clone()` into owned `string` callees.
//!
//! Ecosystem `wj-cli-args`: `has_long_flag` calls `is_long_flag_token(token, name)` inside
//! `for token in rest`. Codegen emitted `is_long_flag_token(&token, …)` (E0308).

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
fn for_loop_borrowed_item_clones_into_owned_string_helper() {
    let source = r#"
fn consumes(token: string) -> bool {
    token == "x" || strings.starts_with(token, "--")
}

pub fn scan(items: Vec<string>) -> bool {
    for item in items {
        if consumes(item) {
            return true
        }
    }
    false
}
"#;
    let generated = test_utils::compile_single(source);
    let clones_into_owned = generated.contains("consumes(item.clone())")
        || generated.contains("consumes(item.to_string())");
    let consumes_owned_loop_elem = generated.contains("for item in items")
        && generated.contains("consumes(item)")
        && !generated.contains("consumes(&item)");
    assert!(
        clones_into_owned || consumes_owned_loop_elem,
        "owned string helper must receive owned value from for-loop elem (clone if borrowed, move if consuming), got:\n{generated}"
    );
    assert!(
        !generated.contains("consumes(&item)"),
        "must not pass &String into owned string formal, got:\n{generated}"
    );
}
