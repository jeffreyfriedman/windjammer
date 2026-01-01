// TDD Tests for expression helper functions (Phase 8)
// These tests are written BEFORE extracting the functions
//
// UPDATED: Now using AST builder functions!

use windjammer::codegen::rust::expression_helpers::{is_const_evaluable, is_reference_expression};
use windjammer::parser::ast::builders::*;
use windjammer::parser::UnaryOp;
use windjammer::test_utils::*;

// Arena-allocating wrappers
fn alloc_var(name: &str) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_var(name))
}

fn alloc_int(n: i64) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_int(n))
}

fn alloc_string(s: &str) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_string(s))
}

fn alloc_bool(b: bool) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_bool(b))
}

fn alloc_unary(op: windjammer::parser::UnaryOp, operand: &'static windjammer::parser::Expression<'static>) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_unary(op, operand))
}

fn alloc_not(operand: &'static windjammer::parser::Expression<'static>) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_not(operand))
}

fn alloc_neg(operand: &'static windjammer::parser::Expression<'static>) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_neg(operand))
}

fn alloc_add(left: &'static windjammer::parser::Expression<'static>, right: &'static windjammer::parser::Expression<'static>) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_add(left, right))
}

fn alloc_tuple(elements: Vec<&'static windjammer::parser::Expression<'static>>) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_tuple(elements))
}

// =============================================================================
// is_reference_expression Tests
// =============================================================================

#[test]
fn test_is_reference_expression_ref() {
    // &x → true
    let expr = alloc_unary(UnaryOp::Ref, alloc_var("x"));
    assert!(is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_mut_ref() {
    // &mut x → true
    let expr = alloc_unary(UnaryOp::MutRef, alloc_var("x"));
    assert!(is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_not() {
    // !x → false
    let expr = alloc_not(alloc_var("x"));
    assert!(!is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_identifier() {
    // x → false
    let expr = alloc_var("x");
    assert!(!is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_literal() {
    // 42 → false
    let expr = alloc_int(42);
    assert!(!is_reference_expression(&expr));
}

// =============================================================================
// is_const_evaluable Tests
// =============================================================================

#[test]
fn test_is_const_evaluable_literal_int() {
    // 42 → true
    let expr = alloc_int(42);
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_literal_string() {
    // "hello" → true
    let expr = alloc_string("hello");
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_literal_bool() {
    // true → true
    let expr = alloc_bool(true);
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_binary_const() {
    // 1 + 2 → true
    let expr = alloc_add(alloc_int(1), alloc_int(2));
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_binary_non_const() {
    // x + 2 → false
    let expr = alloc_add(alloc_var("x"), alloc_int(2));
    assert!(!is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_unary_const() {
    // -5 → true
    let expr = alloc_neg(alloc_int(5));
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_unary_non_const() {
    // -x → false
    let expr = alloc_neg(alloc_var("x"));
    assert!(!is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_identifier() {
    // x → false
    let expr = alloc_var("x");
    assert!(!is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_complex() {
    // (1 + 2) * 3 → true
    let expr = expr_mul(alloc_add(alloc_int(1), alloc_int(2)), alloc_int(3));
    assert!(is_const_evaluable(&expr));
}
