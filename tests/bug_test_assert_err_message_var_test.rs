//! `assert(false, err_var)` must not emit Rust `assert!(false, err_var)` (E0277).
//!
//! Ecosystem `wj-timefmt` hit this when matching `Err(e) => assert(false, e)`.
//! Workaround: use a string literal message.

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
fn assert_err_message_var_must_not_emit_rust_assert_macro() {
    let generated = test_utils::compile_single(
        r#"
@test
fn test_fail_with_err() {
    let e = "boom"
    assert(false, e)
}
"#,
    );

    assert!(
        !generated.contains("assert!(false, e)"),
        "assert(false, err_var) must not become assert!(false, err_var):\n{generated}"
    );
}
