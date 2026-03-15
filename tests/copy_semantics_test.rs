//! TDD Tests for Copy Semantics Layer
//!
//! Validates Copy semantics application to ownership tracking:
//! - is_type_copy
//! - effective_ownership
//! - needs_explicit_deref

use windjammer::analyzer::OwnershipMode;
use windjammer::codegen::rust::copy_semantics::{CopySemantics, DerefContext};
use windjammer::parser::Type;

// =============================================================================
// is_type_copy Tests
// =============================================================================

#[test]
fn test_primitives_are_copy() {
    let semantics = CopySemantics::new();
    assert!(semantics.is_type_copy(&Type::Int32));
    assert!(semantics.is_type_copy(&Type::Float));
    assert!(semantics.is_type_copy(&Type::Bool));
    assert!(semantics.is_type_copy(&Type::Int));
    assert!(semantics.is_type_copy(&Type::Uint));
}

#[test]
fn test_string_not_copy() {
    let semantics = CopySemantics::new();
    assert!(!semantics.is_type_copy(&Type::String));
}

#[test]
fn test_tuple_all_copy_is_copy() {
    let semantics = CopySemantics::new();
    let tuple = Type::Tuple(vec![Type::Int32, Type::Float, Type::Bool]);
    assert!(semantics.is_type_copy(&tuple));
}

#[test]
fn test_tuple_with_noncopy_not_copy() {
    let semantics = CopySemantics::new();
    let tuple = Type::Tuple(vec![Type::Int32, Type::String]);
    assert!(!semantics.is_type_copy(&tuple));
}

#[test]
fn test_custom_type_in_registry() {
    let mut semantics = CopySemantics::new();
    semantics.register_copy_type("Entity");
    assert!(semantics.is_type_copy(&Type::Custom("Entity".to_string())));
}

#[test]
fn test_custom_primitive_names_copy() {
    let semantics = CopySemantics::new();
    assert!(semantics.is_type_copy(&Type::Custom("i32".to_string())));
    assert!(semantics.is_type_copy(&Type::Custom("i64".to_string())));
    assert!(semantics.is_type_copy(&Type::Custom("f32".to_string())));
    assert!(semantics.is_type_copy(&Type::Custom("f64".to_string())));
    assert!(semantics.is_type_copy(&Type::Custom("usize".to_string())));
    assert!(semantics.is_type_copy(&Type::Custom("char".to_string())));
}

#[test]
fn test_custom_non_copy() {
    let semantics = CopySemantics::new();
    assert!(!semantics.is_type_copy(&Type::Custom("String".to_string())));
    assert!(!semantics.is_type_copy(&Type::Custom("Vec".to_string())));
}

#[test]
fn test_option_copy_inner_is_copy() {
    let semantics = CopySemantics::new();
    assert!(semantics.is_type_copy(&Type::Option(Box::new(Type::Int32))));
    assert!(semantics.is_type_copy(&Type::Option(Box::new(Type::Bool))));
}

#[test]
fn test_option_non_copy_inner_not_copy() {
    let semantics = CopySemantics::new();
    assert!(!semantics.is_type_copy(&Type::Option(Box::new(Type::String))));
}

#[test]
fn test_array_copy_inner_is_copy() {
    let semantics = CopySemantics::new();
    assert!(semantics.is_type_copy(&Type::Array(Box::new(Type::Int32), 4)));
}

#[test]
fn test_vec_never_copy() {
    let semantics = CopySemantics::new();
    assert!(!semantics.is_type_copy(&Type::Vec(Box::new(Type::Int))));
}

#[test]
fn test_reference_to_copy_is_copy() {
    // &i32, &mut i32: check pointee for Copy (used in binary ops, comparisons)
    let semantics = CopySemantics::new();
    assert!(semantics.is_type_copy(&Type::Reference(Box::new(Type::Int32))));
    assert!(semantics.is_type_copy(&Type::Reference(Box::new(Type::Custom(
        "i32".to_string()
    )))));
    assert!(semantics.is_type_copy(&Type::MutableReference(Box::new(
        Type::Float
    ))));
}

#[test]
fn test_reference_to_noncopy_not_copy() {
    let semantics = CopySemantics::new();
    assert!(!semantics.is_type_copy(&Type::Reference(Box::new(Type::String))));
}

#[test]
fn test_qualified_custom_type_in_registry() {
    let mut semantics = CopySemantics::new();
    semantics.register_copy_type("module::Entity");
    assert!(semantics.is_type_copy(&Type::Custom("module::Entity".to_string())));
    assert!(semantics.is_type_copy(&Type::Custom("Entity".to_string())));
}

// =============================================================================
// effective_ownership Tests
// =============================================================================

#[test]
fn test_borrowed_copy_becomes_owned() {
    let semantics = CopySemantics::new();
    let result = semantics.effective_ownership(OwnershipMode::Borrowed, &Type::Int32);
    assert_eq!(result, OwnershipMode::Owned);
}

#[test]
fn test_mut_borrowed_copy_becomes_owned() {
    let semantics = CopySemantics::new();
    let result = semantics.effective_ownership(OwnershipMode::MutBorrowed, &Type::Float);
    assert_eq!(result, OwnershipMode::Owned);
}

#[test]
fn test_borrowed_noncopy_stays_borrowed() {
    let semantics = CopySemantics::new();
    let result = semantics.effective_ownership(OwnershipMode::Borrowed, &Type::String);
    assert_eq!(result, OwnershipMode::Borrowed);
}

#[test]
fn test_mut_borrowed_noncopy_stays_mut_borrowed() {
    let semantics = CopySemantics::new();
    let result = semantics.effective_ownership(OwnershipMode::MutBorrowed, &Type::String);
    assert_eq!(result, OwnershipMode::MutBorrowed);
}

#[test]
fn test_owned_stays_owned() {
    let semantics = CopySemantics::new();
    let result = semantics.effective_ownership(OwnershipMode::Owned, &Type::Int32);
    assert_eq!(result, OwnershipMode::Owned);
}

#[test]
fn test_borrowed_reference_to_copy_becomes_owned() {
    // a: &i32 in a + b → effective ownership Owned (Rust auto-copies)
    let semantics = CopySemantics::new();
    let ref_i32 = Type::Reference(Box::new(Type::Int32));
    let result = semantics.effective_ownership(OwnershipMode::Borrowed, &ref_i32);
    assert_eq!(result, OwnershipMode::Owned);
}

#[test]
fn test_borrowed_tuple_copy_becomes_owned() {
    let semantics = CopySemantics::new();
    let tuple = Type::Tuple(vec![Type::Int32, Type::Bool]);
    let result = semantics.effective_ownership(OwnershipMode::Borrowed, &tuple);
    assert_eq!(result, OwnershipMode::Owned);
}

#[test]
fn test_borrowed_custom_registry_copy_becomes_owned() {
    let mut semantics = CopySemantics::new();
    semantics.register_copy_type("Vec3");
    let result = semantics.effective_ownership(
        OwnershipMode::Borrowed,
        &Type::Custom("Vec3".to_string()),
    );
    assert_eq!(result, OwnershipMode::Owned);
}

// =============================================================================
// needs_explicit_deref Tests
// =============================================================================

#[test]
fn test_copy_in_comparison_no_deref() {
    let semantics = CopySemantics::new();
    let needs = semantics.needs_explicit_deref(
        DerefContext::Comparison,
        OwnershipMode::Borrowed,
        &Type::Int32,
    );
    assert!(!needs); // Rust auto-derefs
}

#[test]
fn test_copy_in_struct_literal_no_deref() {
    let semantics = CopySemantics::new();
    let needs = semantics.needs_explicit_deref(
        DerefContext::StructLiteral,
        OwnershipMode::Borrowed,
        &Type::Float,
    );
    assert!(!needs); // Rust auto-copies
}

#[test]
fn test_noncopy_in_struct_literal_no_deref() {
    let semantics = CopySemantics::new();
    let needs = semantics.needs_explicit_deref(
        DerefContext::StructLiteral,
        OwnershipMode::Borrowed,
        &Type::String,
    );
    assert!(!needs); // Need .clone(), not *
}

#[test]
fn test_owned_never_needs_deref() {
    let semantics = CopySemantics::new();
    let needs = semantics.needs_explicit_deref(
        DerefContext::Standalone,
        OwnershipMode::Owned,
        &Type::Int32,
    );
    assert!(!needs);
}

#[test]
fn test_copy_in_method_call_no_deref() {
    let semantics = CopySemantics::new();
    let needs = semantics.needs_explicit_deref(
        DerefContext::MethodCall,
        OwnershipMode::Borrowed,
        &Type::Int32,
    );
    assert!(!needs);
}

#[test]
fn test_copy_in_field_access_no_deref() {
    let semantics = CopySemantics::new();
    let needs = semantics.needs_explicit_deref(
        DerefContext::FieldAccess,
        OwnershipMode::Borrowed,
        &Type::Bool,
    );
    assert!(!needs);
}

#[test]
fn test_copy_in_binary_op_no_deref() {
    let semantics = CopySemantics::new();
    let needs = semantics.needs_explicit_deref(
        DerefContext::BinaryOp,
        OwnershipMode::Borrowed,
        &Type::Int32,
    );
    assert!(!needs);
}

#[test]
fn test_copy_in_function_arg_no_deref() {
    let semantics = CopySemantics::new();
    let needs = semantics.needs_explicit_deref(
        DerefContext::FunctionArg,
        OwnershipMode::Borrowed,
        &Type::Float,
    );
    assert!(!needs);
}

#[test]
fn test_set_copy_types_registry() {
    let mut semantics = CopySemantics::new();
    let mut registry = std::collections::HashSet::new();
    registry.insert("MyCopy".to_string());
    registry.insert("Other::Copy".to_string());
    semantics.set_copy_types(&registry);
    assert!(semantics.is_type_copy(&Type::Custom("MyCopy".to_string())));
    assert!(semantics.is_type_copy(&Type::Custom("Copy".to_string())));
}

#[test]
fn test_deref_context_equality() {
    assert_eq!(DerefContext::Comparison, DerefContext::Comparison);
    assert_ne!(DerefContext::Comparison, DerefContext::MethodCall);
}

#[test]
fn test_copy_semantics_default() {
    let semantics = CopySemantics::default();
    assert!(semantics.is_type_copy(&Type::Int32));
    assert!(!semantics.is_type_copy(&Type::String));
}
