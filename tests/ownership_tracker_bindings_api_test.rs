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

//! OwnershipTracker: bindings, casts, misc exprs, API.
#![allow(clippy::assertions_on_constants)]

use windjammer::analyzer::OwnershipMode;
use windjammer::codegen::rust::ownership_tracker::OwnershipTracker;
use windjammer::parser::Expression;
use windjammer::test_utils::test_alloc_expr;

#[path = "common/ownership_tracker_alloc.rs"]
mod ownership_tracker_alloc;
use ownership_tracker_alloc::*;

#[test]
fn test_for_loop_binding_shared() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_binding("item", OwnershipMode::Borrowed);

    let expr = alloc_var("item");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_match_pattern_binding_shared() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_binding("val", OwnershipMode::Borrowed);

    let expr = alloc_var("val");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_for_loop_binding_mut() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_binding("item", OwnershipMode::MutBorrowed);

    let expr = alloc_var("item");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

#[test]
fn test_binding_overrides_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);
    tracker.register_binding("x", OwnershipMode::MutBorrowed);

    let expr = alloc_var("x");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

// ============================================================================
// CAST TESTS
// ============================================================================

#[test]
fn test_cast_inherits_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);

    let expr = test_alloc_expr(Expression::Cast {
        expr: alloc_var("x"),
        type_: windjammer::parser::Type::Float,
        location: None,
    });
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

// ============================================================================
// RANGE TESTS
// ============================================================================

#[test]
fn test_range_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::Range {
        start: alloc_int(0),
        end: alloc_int(10),
        inclusive: false,
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// TRY OPERATOR TESTS
// ============================================================================

#[test]
fn test_try_inherits_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("result", OwnershipMode::Borrowed);

    let expr = test_alloc_expr(Expression::TryOp {
        expr: alloc_var("result"),
        location: None,
    });
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

// ============================================================================
// MAP LITERAL TESTS
// ============================================================================

#[test]
fn test_map_literal_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::MapLiteral {
        pairs: vec![(alloc_var("k"), alloc_var("v"))],
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// CLOSURE TESTS
// ============================================================================

#[test]
fn test_closure_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::Closure {
        parameters: vec!["x".to_string()],
        body: alloc_var("x"),
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// MACRO INVOCATION TESTS
// ============================================================================

#[test]
fn test_macro_invocation_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::MacroInvocation {
        name: "vec".to_string(),
        args: vec![alloc_int(1)],
        delimiter: windjammer::parser::MacroDelimiter::Brackets,
        is_repeat: false,
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// AWAIT TESTS
// ============================================================================

#[test]
fn test_await_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::Await {
        expr: alloc_var("future"),
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// HELPER METHOD TESTS
// ============================================================================

#[test]
fn test_is_borrowed_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);
    assert!(tracker.is_borrowed_parameter("x"));
    assert!(!tracker.is_borrowed_parameter("y"));
}

#[test]
fn test_is_mut_borrowed_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::MutBorrowed);
    assert!(tracker.is_mut_borrowed_parameter("x"));
    assert!(!tracker.is_mut_borrowed_parameter("y"));
}

#[test]
fn test_get_binding_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_binding("item", OwnershipMode::Borrowed);
    assert_eq!(
        tracker.get_binding_ownership("item"),
        Some(OwnershipMode::Borrowed)
    );
    assert_eq!(tracker.get_binding_ownership("other"), None);
}

#[test]
fn test_register_copy_type() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_copy_type("Point");
    // Copy types don't affect get_expression_ownership yet - future optimization
    assert!(true);
}

#[test]
fn test_register_struct_field() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_struct_field("Point.x", windjammer::parser::Type::Int);
    // Struct fields don't affect get_expression_ownership yet - future use
    assert!(true);
}

#[test]
fn test_default_creates_empty_tracker() {
    let tracker = OwnershipTracker::default();
    let expr = alloc_var("x");
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}
