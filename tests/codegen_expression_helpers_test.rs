// TDD Tests for expression helper functions (Phase 8)
// These tests are written BEFORE extracting the functions
//
// UPDATED: Now using AST builder functions!

use windjammer::codegen::rust::expression_helpers::{is_const_evaluable, is_reference_expression};
use windjammer::parser::ast::builders::*;
use windjammer::parser::UnaryOp;

// =============================================================================
// is_reference_expression Tests
// =============================================================================

#[test]
fn test_is_reference_expression_ref() {
    // &x → true
    let expr = expr_unary(UnaryOp::Ref, expr_var("x"));
    assert!(is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_mut_ref() {
    // &mut x → true
    let expr = expr_unary(UnaryOp::MutRef, expr_var("x"));
    assert!(is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_not() {
    // !x → false
    let expr = expr_not(expr_var("x"));
    assert!(!is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_identifier() {
    // x → false
    let expr = expr_var("x");
    assert!(!is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_literal() {
    // 42 → false
    let expr = expr_int(42);
    assert!(!is_reference_expression(&expr));
}

// =============================================================================
// is_const_evaluable Tests
// =============================================================================

#[test]
fn test_is_const_evaluable_literal_int() {
    // 42 → true
    let expr = expr_int(42);
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_literal_string() {
    // "hello" → true
    let expr = expr_string("hello");
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_literal_bool() {
    // true → true
    let expr = expr_bool(true);
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_binary_const() {
    // 1 + 2 → true
    let expr = expr_add(expr_int(1), expr_int(2));
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_binary_non_const() {
    // x + 2 → false
    let expr = expr_add(expr_var("x"), expr_int(2));
    assert!(!is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_unary_const() {
    // -5 → true
    let expr = expr_neg(expr_int(5));
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_unary_non_const() {
    // -x → false
    let expr = expr_neg(expr_var("x"));
    assert!(!is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_identifier() {
    // x → false
    let expr = expr_var("x");
    assert!(!is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_complex() {
    // (1 + 2) * 3 → true
    let expr = expr_mul(expr_add(expr_int(1), expr_int(2)), expr_int(3));
    assert!(is_const_evaluable(&expr));
}
