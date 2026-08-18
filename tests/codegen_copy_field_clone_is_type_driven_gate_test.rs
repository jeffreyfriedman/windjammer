#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! FAILING REPRO — auto-clone must follow field *types*, never field *names*.
//!
//! A `string` field named `x` is non-Copy. Reusing it must emit `.clone()` even
//! though `x` looks like a coordinate. A Copy `int` field named `payload_count`
//! must not clone. No `matches!(field, "x" | "y" | …)` heuristic.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn non_copy_field_named_x_must_clone_on_reuse() {
    let source = r#"
pub struct Label {
    pub x: string,
}

pub fn dup_x(label: Label) -> (string, string) {
    let a = label.x
    let b = label.x
    (a, b)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("label.x.clone()") || generated.contains(".x.clone()"),
        "non-Copy field named x must auto-clone on reuse (type-driven, not name).\nGenerated:\n{generated}"
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}

#[test]
fn copy_field_with_noncanonical_name_must_not_clone() {
    let source = r#"
pub struct Stats {
    pub payload_count: int,
}

pub fn dup_count(stats: Stats) -> (int, int) {
    let a = stats.payload_count
    let b = stats.payload_count
    (a, b)
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("payload_count.clone()"),
        "Copy int field must not clone even when the name is not x/y/id.\nGenerated:\n{generated}"
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}
