//! `std::mime` must codegen to `windjammer_runtime::mime` and `cargo check`.

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
fn std_mime_from_extension_codegen_resolves() {
    let source = r#"
use std::mime

pub fn lookup(ext: string) -> string {
    mime.from_extension(ext)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["mime::from_extension"]);
}

#[test]
fn std_mime_constants_codegen_resolves() {
    let source = r#"
use std::mime

pub fn json_type() -> string {
    mime.APPLICATION_JSON
}
"#;
    let generated = test_utils::assert_stdlib_runtime_links(source, &[]);
    assert!(
        !generated.contains("mime.APPLICATION_JSON"),
        "must not use dot for module constants, got:\n{generated}"
    );
    assert!(
        generated.contains("APPLICATION_JSON"),
        "must reference APPLICATION_JSON, got:\n{generated}"
    );
}
