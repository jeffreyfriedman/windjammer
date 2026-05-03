//! TDD: Generic type parameter propagation in Rust codegen
//!
//! Bug: E0425 - "cannot find type 'T' in this scope" (19 errors in windjammer-game)
//! Root cause: Codegen doesn't preserve generic type parameters when generating Rust
//!
//! Philosophy: "Compiler does hard work" - type parameter propagation is mechanical

#[path = "test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_function_preserves_type_parameter() {
    let generated =
        test_utils::compile_fixture("generic_type_propagation").expect("Compilation failed");

    assert!(
        generated.contains("fn identity<T>") || generated.contains("pub fn identity<T>"),
        "Generic function should preserve <T> in signature. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("value: T") || generated.contains("value: T)"),
        "Parameter type T should be preserved. Generated:\n{}",
        generated
    );

    test_utils::verify_rust_compiles(&generated).expect("Generated Rust should compile");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_struct_preserves_type_parameter() {
    let generated = test_utils::compile_fixture("generic_struct_impl").expect("Compilation failed");

    // Verify struct has <T>
    assert!(
        generated.contains("struct Container<T>"),
        "Generic struct should preserve <T>. Generated:\n{}",
        generated
    );

    // Verify impl has <T>
    assert!(
        generated.contains("impl<T> Container<T>"),
        "Generic impl should preserve impl<T>. Generated:\n{}",
        generated
    );

    // Verify method return type uses T
    assert!(
        generated.contains("-> Container<T>") || generated.contains("-> T"),
        "Method return types should use T. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_impl_method_preserves_type_parameter() {
    let generated = test_utils::compile_fixture("generic_method").expect("Compilation failed");

    // Method add_entity has its own <T> (not from impl - Scene has no type params)
    assert!(
        generated.contains("fn add_entity<T>") || generated.contains("pub fn add_entity<T>"),
        "Method with own type param should preserve <T>. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("_entity: T") || generated.contains("entity: T"),
        "Parameter type T should be preserved. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_function_with_wrapping_decorator_preserves_type_parameter() {
    // When a generic function has @timeout, @bench, etc., it goes through
    // generate_function_with_wrapping - that path must also emit <T>
    let generated = test_utils::compile_fixture("generic_with_test").expect("Compilation failed");

    assert!(
        generated.contains("fn identity<T>") || generated.contains("pub fn identity<T>"),
        "Generic function with wrapping decorator should preserve <T>. Generated:\n{}",
        generated
    );
}
