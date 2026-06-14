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

// Tests for automatic method argument conversion
// Windjammer Philosophy: The compiler does the work, not the developer

#[path = "common/test_utils.rs"]
mod test_utils;

/// Helper to compile a test fixture and return the generated Rust code
// ============================================================================
// contains() method - should auto-add & for the search argument
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_contains_adds_reference() {
    let generated =
        test_utils::compile_fixture("method_arg_conversion").expect("Compilation failed");

    // has_item / contains: may be `contains(&item)` or a custom `has_item(&...)` with coercions
    assert!(
        generated.contains("contains(&item)")
            || generated.contains("has_item(&")
            || generated.contains(".contains(&"),
        "Should borrow for contains/has_item argument: {}",
        generated
    );
}

// ============================================================================
// push() method - should handle ownership correctly
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_with_owned_string() {
    let generated =
        test_utils::compile_fixture("method_arg_conversion").expect("Compilation failed");

    // add_item uses push() with owned string
    assert!(
        generated.contains("push(item)"),
        "Should handle push() with owned value: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_string_literal() {
    let generated =
        test_utils::compile_fixture("method_arg_conversion").expect("Compilation failed");

    // add_hello uses push("hello"), should convert to String
    assert!(
        generated.contains("push(\"hello\".to_string())"),
        "Should convert string literal for push(): {}",
        generated
    );
}

// ============================================================================
// String methods - starts_with / ends_with
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_starts_with() {
    let generated =
        test_utils::compile_fixture("method_arg_conversion").expect("Compilation failed");

    // check_prefix uses starts_with()
    assert!(
        generated.contains("starts_with("),
        "Should handle starts_with(): {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ends_with_with_literal() {
    let generated =
        test_utils::compile_fixture("method_arg_conversion").expect("Compilation failed");

    // is_rust_file uses ends_with(".rs")
    assert!(
        generated.contains("ends_with(\".rs\")"),
        "Should handle ends_with() with literal: {}",
        generated
    );
}

// ============================================================================
// Combined: verify the entire fixture compiles
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_fixture_compiles_successfully() {
    let generated =
        test_utils::compile_fixture("method_arg_conversion").expect("Compilation failed");

    // Debug output to understand CI failures
    if generated.is_empty() {
        eprintln!("WARNING: Generated code is EMPTY!");
    } else {
        eprintln!("Generated code length: {} bytes", generated.len());
    }

    // Basic sanity check - should have the struct
    assert!(
        generated.contains("struct ItemList"),
        "Should generate ItemList struct (length={}): {}",
        generated.len(),
        if generated.len() > 500 {
            &generated[..500]
        } else {
            &generated
        }
    );

    // Should have impl block
    assert!(
        generated.contains("impl ItemList"),
        "Should generate impl block: {}",
        generated
    );
}
