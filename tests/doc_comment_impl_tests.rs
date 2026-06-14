#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! Tests for doc comments inside impl blocks and trait declarations.

#[path = "common/test_utils.rs"]
mod test_utils;

/// Helper to compile a test fixture and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_doc_comment_in_impl_block() {
    let generated = test_utils::compile_fixture("doc_comments_impl").expect("Compilation failed");

    // Check that doc comments appear in the generated Rust code
    assert!(
        generated.contains("/// Creates a new Point at the origin."),
        "Missing doc comment for zero(). Generated code:\n{}",
        generated
    );
    assert!(
        generated.contains("/// Creates a new Point with the given coordinates."),
        "Missing doc comment for new(). Generated code:\n{}",
        generated
    );
    assert!(
        generated.contains("/// Calculates the distance from the origin."),
        "Missing doc comment for distance_from_origin(). Generated code:\n{}",
        generated
    );

    println!("✓ Doc comments in impl blocks work");
}
