//! Comprehensive tests for RustCoercionRules
//!
//! TDD: Tests for Rust's automatic coercion rules (auto-deref, auto-copy).

use windjammer::analyzer::OwnershipMode;
use windjammer::codegen::rust::copy_semantics::DerefContext;
use windjammer::codegen::rust::rust_coercion_rules::{Coercion, RustCoercionRules};

// ============================================================================
// rust_auto_derefs tests
// ============================================================================

#[test]
fn test_auto_deref_in_comparison() {
    assert!(RustCoercionRules::rust_auto_derefs(DerefContext::Comparison));
}

#[test]
fn test_auto_deref_in_method_call() {
    assert!(RustCoercionRules::rust_auto_derefs(DerefContext::MethodCall));
}

#[test]
fn test_auto_deref_in_field_access() {
    assert!(RustCoercionRules::rust_auto_derefs(DerefContext::FieldAccess));
}

#[test]
fn test_auto_deref_in_binary_op() {
    assert!(RustCoercionRules::rust_auto_derefs(DerefContext::BinaryOp));
}

#[test]
fn test_no_auto_deref_in_struct_literal() {
    assert!(!RustCoercionRules::rust_auto_derefs(DerefContext::StructLiteral));
}

#[test]
fn test_no_auto_deref_in_function_arg() {
    assert!(!RustCoercionRules::rust_auto_derefs(DerefContext::FunctionArg));
}

#[test]
fn test_no_auto_deref_in_standalone() {
    assert!(!RustCoercionRules::rust_auto_derefs(DerefContext::Standalone));
}

// ============================================================================
// rust_auto_copies tests
// ============================================================================

#[test]
fn test_auto_copy_for_copy_in_comparison() {
    assert!(RustCoercionRules::rust_auto_copies(DerefContext::Comparison, true));
}

#[test]
fn test_auto_copy_for_copy_in_binary_op() {
    assert!(RustCoercionRules::rust_auto_copies(DerefContext::BinaryOp, true));
}

#[test]
fn test_auto_copy_for_copy_in_struct_literal() {
    assert!(RustCoercionRules::rust_auto_copies(DerefContext::StructLiteral, true));
}

#[test]
fn test_auto_copy_for_copy_in_function_arg() {
    assert!(RustCoercionRules::rust_auto_copies(DerefContext::FunctionArg, true));
}

#[test]
fn test_no_auto_copy_for_noncopy() {
    assert!(!RustCoercionRules::rust_auto_copies(DerefContext::Comparison, false));
}

#[test]
fn test_no_auto_copy_for_copy_in_standalone() {
    assert!(!RustCoercionRules::rust_auto_copies(DerefContext::Standalone, true));
}

// ============================================================================
// required_coercion tests - same ownership
// ============================================================================

#[test]
fn test_owned_to_owned_no_coercion() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::Owned,
        OwnershipMode::Owned,
        true,
        DerefContext::Standalone,
    );
    assert_eq!(coercion, Coercion::None);
}

#[test]
fn test_borrowed_to_borrowed_no_coercion() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::Borrowed,
        OwnershipMode::Borrowed,
        true,
        DerefContext::Standalone,
    );
    assert_eq!(coercion, Coercion::None);
}

#[test]
fn test_mutborrowed_to_mutborrowed_no_coercion() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::MutBorrowed,
        OwnershipMode::MutBorrowed,
        false,
        DerefContext::FunctionArg,
    );
    assert_eq!(coercion, Coercion::None);
}

// ============================================================================
// required_coercion tests - Borrowed/MutBorrowed → Owned
// ============================================================================

#[test]
fn test_borrowed_copy_to_owned_in_comparison_no_coercion() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::Borrowed,
        OwnershipMode::Owned,
        true,
        DerefContext::Comparison,
    );
    assert_eq!(coercion, Coercion::None);
}

#[test]
fn test_borrowed_copy_to_owned_in_struct_needs_nothing() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::Borrowed,
        OwnershipMode::Owned,
        true,
        DerefContext::StructLiteral,
    );
    assert_eq!(coercion, Coercion::None);
}

#[test]
fn test_borrowed_copy_to_owned_in_standalone_needs_deref() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::Borrowed,
        OwnershipMode::Owned,
        true,
        DerefContext::Standalone,
    );
    assert_eq!(coercion, Coercion::Deref);
}

#[test]
fn test_borrowed_noncopy_to_owned_needs_clone() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::Borrowed,
        OwnershipMode::Owned,
        false,
        DerefContext::StructLiteral,
    );
    assert_eq!(coercion, Coercion::Clone);
}

#[test]
fn test_mutborrowed_copy_to_owned_in_comparison_no_coercion() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::MutBorrowed,
        OwnershipMode::Owned,
        true,
        DerefContext::Comparison,
    );
    assert_eq!(coercion, Coercion::None);
}

#[test]
fn test_mutborrowed_noncopy_to_owned_needs_clone() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::MutBorrowed,
        OwnershipMode::Owned,
        false,
        DerefContext::FunctionArg,
    );
    assert_eq!(coercion, Coercion::Clone);
}

// ============================================================================
// required_coercion tests - Owned → Borrowed
// ============================================================================

#[test]
fn test_owned_to_borrowed_needs_borrow() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::Owned,
        OwnershipMode::Borrowed,
        true,
        DerefContext::FunctionArg,
    );
    assert_eq!(coercion, Coercion::Borrow);
}

#[test]
fn test_owned_to_mutborrowed_needs_borrowmut() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::Owned,
        OwnershipMode::MutBorrowed,
        true,
        DerefContext::FunctionArg,
    );
    assert_eq!(coercion, Coercion::BorrowMut);
}

// ============================================================================
// required_coercion tests - Borrow ↔ MutBorrow
// ============================================================================

#[test]
fn test_mutborrowed_to_borrowed_no_coercion() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::MutBorrowed,
        OwnershipMode::Borrowed,
        false,
        DerefContext::FunctionArg,
    );
    assert_eq!(coercion, Coercion::None);
}

#[test]
fn test_borrowed_to_mutborrowed_no_coercion() {
    let coercion = RustCoercionRules::required_coercion(
        OwnershipMode::Borrowed,
        OwnershipMode::MutBorrowed,
        true,
        DerefContext::Standalone,
    );
    assert_eq!(coercion, Coercion::None);
}
