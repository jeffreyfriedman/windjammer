//! Owned helper return into demoted `&str` formal must auto-borrow.
//!
//! Ecosystem `wj-auth-api` adapter:
//! ```
//! fn method_label(method: HttpMethod) -> string { "GET" }
//! app.handle(method_label(req.method), path, ...)
//! ```
//! Tip demotes `handle`'s first formal to `&str` but emits owned `String`
//! from `method_label` → E0308 expected `&str`, found `String`.

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
fn owned_helper_into_demoted_str_formal_must_auto_borrow() {
    let source = r#"
fn label() -> string {
    "GET"
}

pub fn take_method(method: string) -> int {
    0
}

pub fn sample() -> int {
    take_method(label())
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    // If `method` demotes to &str, call site must borrow helper result.
    if generated.contains("method: &str") || generated.contains("method:&str") {
        assert!(
            ok,
            "owned helper into demoted &str formal must compile, got:\n{generated}"
        );
        assert!(
            generated.contains("take_method(&label()")
                || generated.contains("take_method(&(label())")
                || generated.contains("let") && generated.contains("&"),
            "expected auto-borrow of helper result:\n{generated}"
        );
    } else {
        assert!(
            ok,
            "owned string formal path must compile, got:\n{generated}"
        );
    }
}
