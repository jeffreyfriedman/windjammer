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

//! OwnershipTracker: literals, binaries, calls, aggregates.
#![allow(clippy::assertions_on_constants)]

use windjammer::analyzer::OwnershipMode;
use windjammer::codegen::rust::ownership_tracker::OwnershipTracker;
use windjammer::parser::ast::builders::*;
use windjammer::parser::ast::operators::BinaryOp;
use windjammer::test_utils::test_alloc_expr;

#[path = "common/ownership_tracker_alloc.rs"]
mod ownership_tracker_alloc;
use ownership_tracker_alloc::*;

#[test]
fn test_literal_int_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_int(42);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_literal_float_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(expr_float(3.5));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_literal_string_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(expr_string("hello"));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_literal_bool_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(expr_bool(true));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_literal_char_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(expr_char('x'));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// BINARY OPERATION TESTS
// ============================================================================

#[test]
fn test_binary_operation_is_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);

    let expr = alloc_binary(alloc_var("x"), BinaryOp::Add, alloc_int(1));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_binary_eq_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_binary(alloc_var("a"), BinaryOp::Eq, alloc_var("b"));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_binary_and_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_binary(
        test_alloc_expr(expr_bool(true)),
        BinaryOp::And,
        test_alloc_expr(expr_bool(false)),
    );
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// FUNCTION CALL TESTS
// ============================================================================

#[test]
fn test_function_call_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_call(alloc_var("foo"), vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_function_call_with_args_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_call(alloc_var("create"), vec![(None, alloc_int(42))]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// STRUCT/ARRAY/TUPLE LITERAL TESTS
// ============================================================================

#[test]
fn test_struct_literal_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_struct(
        "Point",
        vec![
            ("x".to_string(), alloc_int(0)),
            ("y".to_string(), alloc_int(0)),
        ],
    );
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_array_literal_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_array(vec![alloc_int(1), alloc_int(2)]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_tuple_literal_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_tuple(vec![alloc_int(1), alloc_var("x")]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}
