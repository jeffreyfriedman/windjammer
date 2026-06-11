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

//! TDD Tests for Trait Bound Inference
//!
//! These tests verify that the compiler correctly infers trait bounds
//! for generic type parameters by compiling Windjammer source and
//! checking the generated Rust code.

#[path = "common/test_utils.rs"]
mod test_utils;

/// Helper to compile a Windjammer snippet and get the generated Rust
#[test]
fn test_display_trait_inferred() {
    let source = r#"
fn print_item<T>(item: T) {
    println!("{}", item)
}

fn main() {
    print_item(42)
}
"#;

    let generated = test_utils::compile_single_result(source).expect("Compilation failed");

    // Check that Display bound is inferred
    assert!(
        generated.contains("T: Display") || generated.contains("T: std::fmt::Display"),
        "Expected Display bound in:\n{}",
        generated
    );
}

#[test]
fn test_debug_trait_inferred() {
    let source = r#"
fn debug_item<T>(item: T) {
    println!("{:?}", item)
}

fn main() {
    debug_item(42)
}
"#;

    let generated = test_utils::compile_single_result(source).expect("Compilation failed");

    // Check that Debug bound is inferred
    assert!(
        generated.contains("T: Debug") || generated.contains("T: std::fmt::Debug"),
        "Expected Debug bound in:\n{}",
        generated
    );
}

#[test]
fn test_clone_trait_inferred() {
    let source = r#"
fn dup<T>(item: T) -> T {
    item
}

fn main() {
    let x = dup(42)
    println!("{}", x)
}
"#;

    let generated = test_utils::compile_single_result(source).expect("Compilation failed");

    // Check that Clone bound is inferred
    assert!(
        generated.contains("T: Clone") || generated.contains("Clone"),
        "Expected Clone bound in:\n{}",
        generated
    );
}

#[test]
fn test_multiple_bounds_inferred() {
    let source = r#"
fn clone_and_print<T>(item: T) -> T {
    println!("{:?}", item)
    item
}

fn main() {
    let x = clone_and_print(42)
}
"#;

    let generated = test_utils::compile_single_result(source).expect("Compilation failed");

    // Check that both Clone and Debug bounds are inferred
    assert!(
        generated.contains("Clone") && generated.contains("Debug"),
        "Expected Clone + Debug bounds in:\n{}",
        generated
    );
}

#[test]
fn test_add_operator_trait_inferred() {
    let source = r#"
fn double<T>(x: T) -> T {
    x + x
}

fn main() {
    println!("{}", double(5))
}
"#;

    let generated = test_utils::compile_single_result(source).expect("Compilation failed");

    // Check that Add bound is inferred (with Output = T for same-type operands)
    assert!(
        generated.contains("Add<Output = T>") || generated.contains("Add"),
        "Expected Add bound in:\n{}",
        generated
    );
    // Should also have Copy since x is used twice
    assert!(
        generated.contains("Copy"),
        "Expected Copy bound in:\n{}",
        generated
    );
}
