//! Owned `string` locals must move into owned `string` formals (not `&String`).
//!
//! Ecosystem `wj-url` tests: `let base = "…"; join(base, relative)` emits
//! `join(&base, &relative)` → E0308.

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
fn owned_string_locals_move_into_owned_string_formals() {
    let source = r#"
pub fn join(base: string, relative: string) -> string {
    "${base}/${relative}"
}

pub fn resolve() -> string {
    let base = "https://example.com/a/b"
    let relative = "c"
    join(base, relative)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("join(base, relative)")
            || generated.contains("join(base.clone(), relative.clone())"),
        "owned string locals must move (or clone) into owned formals, got:\n{generated}"
    );
    assert!(
        !generated.contains("join(&base, &relative)"),
        "must not borrow owned locals into owned string formals, got:\n{generated}"
    );
}
