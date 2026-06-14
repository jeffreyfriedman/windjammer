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

//! OwnershipTracker: identifiers, fields, indices.
#![allow(clippy::assertions_on_constants)]

use windjammer::analyzer::OwnershipMode;
use windjammer::codegen::rust::ownership_tracker::OwnershipTracker;

#[path = "common/ownership_tracker_alloc.rs"]
mod ownership_tracker_alloc;
use ownership_tracker_alloc::*;

#[test]
fn test_identifier_borrowed_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("data", OwnershipMode::Borrowed);

    let expr = alloc_var("data");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_identifier_mut_borrowed_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("data", OwnershipMode::MutBorrowed);

    let expr = alloc_var("data");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

#[test]
fn test_identifier_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_var("data");
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_identifier_unknown_param_defaults_to_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);
    let expr = alloc_var("y"); // y not registered
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// FIELD ACCESS TESTS
// ============================================================================

#[test]
fn test_field_access_inherits_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("self", OwnershipMode::Borrowed);

    let expr = alloc_field(alloc_var("self"), "value");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_nested_field_access() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("self", OwnershipMode::Borrowed);

    let expr = alloc_field(alloc_field(alloc_var("self"), "camera"), "position");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_field_access_on_owned_object() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_field(alloc_var("config"), "name");
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_field_access_on_mut_borrowed() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("self", OwnershipMode::MutBorrowed);
    let expr = alloc_field(alloc_var("self"), "count");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

// ============================================================================
// INDEX TESTS
// ============================================================================

#[test]
fn test_index_inherits_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("items", OwnershipMode::Borrowed);

    let expr = alloc_index(alloc_var("items"), alloc_int(0));
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_index_on_owned_vec() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_index(alloc_var("vec"), alloc_int(0));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_index_on_mut_borrowed() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("arr", OwnershipMode::MutBorrowed);
    let expr = alloc_index(alloc_var("arr"), alloc_int(5));
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}
