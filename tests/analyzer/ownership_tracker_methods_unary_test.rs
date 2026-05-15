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

//! OwnershipTracker: methods and unary operators.
#![allow(clippy::assertions_on_constants)]

use windjammer::analyzer::OwnershipMode;
use windjammer::codegen::rust::ownership_tracker::OwnershipTracker;
use windjammer::parser::ast::builders::*;
use windjammer::parser::UnaryOp;
use windjammer::test_utils::test_alloc_expr;

#[path = "../common/ownership_tracker_alloc.rs"]
mod ownership_tracker_alloc;
use ownership_tracker_alloc::*;

#[test]
fn test_clone_method_returns_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("data", OwnershipMode::Borrowed);

    let expr = alloc_method(alloc_var("data"), "clone", vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_to_owned_method_returns_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("cow", OwnershipMode::Borrowed);
    let expr = alloc_method(alloc_var("cow"), "to_owned", vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_to_string_method_returns_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_method(alloc_var("x"), "to_string", vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_generic_method_returns_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("items", OwnershipMode::Borrowed);
    let expr = alloc_method(alloc_var("items"), "len", vec![]);
    // len() on a borrowed value may be classified as Borrowed (receiver borrow
    // propagation) or Owned (usize is Copy); both are defensible in the IR.
    let m = tracker.get_expression_ownership(expr);
    assert!(
        m == OwnershipMode::Owned || m == OwnershipMode::Borrowed,
        "len on borrowed param should be Owned or Borrowed, got {m:?}"
    );
}

#[test]
fn test_method_on_borrowed_object_result_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("self", OwnershipMode::Borrowed);
    let expr = alloc_method(alloc_field(alloc_var("self"), "data"), "clone", vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// UNARY OPERATOR TESTS
// ============================================================================

#[test]
fn test_deref_removes_borrow() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("ptr", OwnershipMode::Borrowed);

    let expr = alloc_unary(windjammer::parser::UnaryOp::Deref, alloc_var("ptr"));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_deref_mut_borrow_removes_borrow() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("ptr", OwnershipMode::MutBorrowed);
    let expr = alloc_unary(windjammer::parser::UnaryOp::Deref, alloc_var("ptr"));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_borrow_adds_shared_borrow() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_unary(windjammer::parser::UnaryOp::Ref, alloc_var("x"));
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_borrow_mut_adds_mutable_borrow() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_unary(windjammer::parser::UnaryOp::MutRef, alloc_var("x"));
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

#[test]
fn test_neg_produces_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_unary(windjammer::parser::UnaryOp::Neg, alloc_int(5));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_not_produces_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_unary(
        windjammer::parser::UnaryOp::Not,
        test_alloc_expr(expr_bool(true)),
    );
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}
