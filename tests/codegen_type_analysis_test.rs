// TDD Tests for type_analysis functions (Phase 8 - Retroactive TDD)

use windjammer::codegen::rust::type_analysis::is_copy_type;
use windjammer::parser::Type;

// =============================================================================
// is_copy_type Tests
// =============================================================================

#[test]
fn test_is_copy_type_primitives() {
    // Primitive types are Copy
    assert!(is_copy_type(&Type::Int));
    assert!(is_copy_type(&Type::Int32));
    assert!(is_copy_type(&Type::Uint));
    assert!(is_copy_type(&Type::Float));
    assert!(is_copy_type(&Type::Bool));
}

#[test]
fn test_is_copy_type_non_copy_primitives() {
    // String is NOT Copy
    assert!(!is_copy_type(&Type::String));
}

#[test]
fn test_is_copy_type_custom_known_copy() {
    // Known Copy types
    assert!(is_copy_type(&Type::Custom("i32".to_string())));
    assert!(is_copy_type(&Type::Custom("i64".to_string())));
    assert!(is_copy_type(&Type::Custom("u32".to_string())));
    assert!(is_copy_type(&Type::Custom("u64".to_string())));
    assert!(is_copy_type(&Type::Custom("f32".to_string())));
    assert!(is_copy_type(&Type::Custom("f64".to_string())));
    assert!(is_copy_type(&Type::Custom("bool".to_string())));
    assert!(is_copy_type(&Type::Custom("char".to_string())));
    assert!(is_copy_type(&Type::Custom("usize".to_string())));
    assert!(is_copy_type(&Type::Custom("isize".to_string())));
}

#[test]
fn test_is_copy_type_custom_non_copy() {
    // String and Vec are NOT Copy
    assert!(!is_copy_type(&Type::Custom("String".to_string())));
    assert!(!is_copy_type(&Type::Custom("Vec".to_string())));
}

#[test]
fn test_is_copy_type_option() {
    // Current implementation: Option is conservatively treated as non-Copy
    // Note: In Rust, Option<T> is Copy if T is Copy, but we're being conservative here
    assert!(!is_copy_type(&Type::Option(Box::new(Type::Int32))));
    assert!(!is_copy_type(&Type::Option(Box::new(Type::Bool))));
    assert!(!is_copy_type(&Type::Option(Box::new(Type::String))));
}

#[test]
fn test_is_copy_type_vec() {
    // Vec is never Copy
    assert!(!is_copy_type(&Type::Vec(Box::new(Type::Int))));
    assert!(!is_copy_type(&Type::Vec(Box::new(Type::String))));
}

#[test]
fn test_is_copy_type_tuple_all_copy() {
    // Tuple of all Copy types is Copy
    assert!(is_copy_type(&Type::Tuple(vec![Type::Int, Type::Bool])));
    assert!(is_copy_type(&Type::Tuple(vec![
        Type::Float,
        Type::Int32,
        Type::Bool
    ])));
}

#[test]
fn test_is_copy_type_tuple_with_non_copy() {
    // Tuple with any non-Copy type is NOT Copy
    assert!(!is_copy_type(&Type::Tuple(vec![Type::Int, Type::String])));
    assert!(!is_copy_type(&Type::Tuple(vec![
        Type::Bool,
        Type::Float,
        Type::Vec(Box::new(Type::Int))
    ])));
}

#[test]
fn test_is_copy_type_tuple_empty() {
    // Empty tuple (unit type) is Copy
    assert!(is_copy_type(&Type::Tuple(vec![])));
}

#[test]
fn test_is_copy_type_array() {
    // Current implementation: Array is conservatively treated as non-Copy
    // Note: In Rust, [T; N] is Copy if T is Copy and N <= 32, but we're being conservative
    assert!(!is_copy_type(&Type::Array(Box::new(Type::Int), 10)));
    assert!(!is_copy_type(&Type::Array(Box::new(Type::Bool), 5)));
    assert!(!is_copy_type(&Type::Array(Box::new(Type::String), 10)));
}

#[test]
fn test_is_copy_type_reference() {
    // References are Copy (regardless of inner type)
    assert!(is_copy_type(&Type::Reference(Box::new(Type::String))));
    assert!(is_copy_type(&Type::Reference(Box::new(Type::Vec(
        Box::new(Type::Int)
    )))));
}

#[test]
fn test_is_copy_type_mut_reference() {
    // Mutable references are NOT Copy (only immutable refs are Copy)
    // This is correct behavior - you can't have multiple &mut references
    assert!(!is_copy_type(&Type::MutableReference(Box::new(
        Type::String
    ))));
    assert!(!is_copy_type(&Type::MutableReference(Box::new(Type::Vec(
        Box::new(Type::Int)
    )))));
}

#[test]
fn test_is_copy_type_unknown() {
    // Unknown custom types are assumed non-Copy
    assert!(!is_copy_type(&Type::Custom("MyStruct".to_string())));
    assert!(!is_copy_type(&Type::Custom("GameObject".to_string())));
}

#[test]
fn test_is_copy_type_result() {
    // Current implementation: Result is conservatively treated as non-Copy
    // Note: In Rust, Result<T, E> is Copy if both T and E are Copy, but we're being conservative
    assert!(!is_copy_type(&Type::Result(
        Box::new(Type::Int32),
        Box::new(Type::Int32)
    )));
    assert!(!is_copy_type(&Type::Result(
        Box::new(Type::String),
        Box::new(Type::String)
    )));
}

#[test]
fn test_is_copy_type_infer() {
    // Type::Infer should be assumed non-Copy (conservative)
    assert!(!is_copy_type(&Type::Infer));
}
