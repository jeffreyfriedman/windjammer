// TDD Tests for Match Arm String Analysis
// Tests written FIRST before extraction!
//
// UPDATED: Now using AST builder functions!

#![allow(clippy::needless_borrow)]

use windjammer::codegen::rust::arm_string_analysis::*;
use windjammer::parser::ast::builders::*;
use windjammer::parser::{Expression, Statement};
use windjammer::test_utils::*;

// Helper wrappers using builders
fn alloc_string(s: &str) -> &'static Expression<'static> {
    test_alloc_expr(expr_string(s))
}

fn alloc_int(n: i64) -> &'static Expression<'static> {
    test_alloc_expr(expr_int(n))
}

fn alloc_block(statements: Vec<&'static Statement<'static>>) -> &'static Expression<'static> {
    test_alloc_expr(expr_block(statements))
}

fn alloc_stmt_expr(expr: &'static Expression<'static>) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_expr(expr))
}

fn alloc_stmt_if(
    condition: &'static Expression<'static>,
    then_block: Vec<&'static Statement<'static>>,
    else_block: Option<Vec<&'static Statement<'static>>>,
) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_if(condition, then_block, else_block))
}

// ============================================================================
// BLOCK WITH IF-ELSE TESTS
// ============================================================================

#[test]
fn test_block_with_if_else_both_string_literals() {
    // { if cond { "yes" } else { "no" } }
    let if_statement = alloc_stmt_if(
        alloc_int(1), // condition (doesn't matter for this test)
        vec![alloc_stmt_expr(alloc_string("yes"))],
        Some(vec![alloc_stmt_expr(alloc_string("no"))]),
    );

    let expr = alloc_block(vec![if_statement]);

    assert!(arm_returns_converted_string(expr));
}

#[test]
fn test_block_with_if_else_then_string_else_int() {
    // { if cond { "yes" } else { 42 } }
    let if_statement = alloc_stmt_if(
        alloc_int(1),
        vec![alloc_stmt_expr(alloc_string("yes"))],
        Some(vec![alloc_stmt_expr(alloc_int(42))]),
    );

    let expr = alloc_block(vec![if_statement]);

    assert!(!arm_returns_converted_string(expr));
}

#[test]
fn test_block_with_if_else_then_int_else_string() {
    // { if cond { 42 } else { "no" } }
    let if_statement = alloc_stmt_if(
        alloc_int(1),
        vec![alloc_stmt_expr(alloc_int(42))],
        Some(vec![alloc_stmt_expr(alloc_string("no"))]),
    );

    let expr = alloc_block(vec![if_statement]);

    assert!(!arm_returns_converted_string(expr));
}

#[test]
fn test_block_with_if_no_else() {
    // { if cond { "yes" } }
    let if_statement = alloc_stmt_if(
        alloc_int(1),
        vec![alloc_stmt_expr(alloc_string("yes"))],
        None,
    );

    let expr = alloc_block(vec![if_statement]);

    assert!(
        !arm_returns_converted_string(expr),
        "If without else should return false"
    );
}

#[test]
fn test_block_with_if_else_both_ints() {
    // { if cond { 1 } else { 2 } }
    let if_statement = alloc_stmt_if(
        alloc_int(1),
        vec![alloc_stmt_expr(alloc_int(1))],
        Some(vec![alloc_stmt_expr(alloc_int(2))]),
    );

    let expr = alloc_block(vec![if_statement]);

    assert!(!arm_returns_converted_string(expr));
}

// ============================================================================
// BLOCK WITH EXPRESSION STATEMENT TESTS
// ============================================================================

#[test]
fn test_block_with_string_literal_expression() {
    // { "hello" }
    let expr = alloc_block(vec![alloc_stmt_expr(alloc_string("hello"))]);

    assert!(arm_returns_converted_string(expr));
}

#[test]
fn test_block_with_int_literal_expression() {
    // { 42 }
    let expr = alloc_block(vec![alloc_stmt_expr(alloc_int(42))]);

    assert!(!arm_returns_converted_string(expr));
}

#[test]
fn test_block_with_nested_block_string_literal() {
    // { { "nested" } }
    let inner_block = alloc_block(vec![alloc_stmt_expr(alloc_string("nested"))]);
    let outer_block = alloc_block(vec![alloc_stmt_expr(inner_block)]);

    assert!(arm_returns_converted_string(&outer_block));
}

#[test]
fn test_block_with_nested_block_int_literal() {
    // { { 42 } }
    let inner_block = alloc_block(vec![alloc_stmt_expr(alloc_int(42))]);
    let outer_block = alloc_block(vec![alloc_stmt_expr(inner_block)]);

    assert!(!arm_returns_converted_string(&outer_block));
}

// ============================================================================
// EMPTY BLOCK TESTS
// ============================================================================

#[test]
fn test_empty_block() {
    // { }
    let expr = alloc_block(vec![]);

    assert!(!arm_returns_converted_string(expr));
}

// ============================================================================
// NON-BLOCK EXPRESSION TESTS
// ============================================================================

#[test]
fn test_non_block_string_literal() {
    // Just "hello" (not in a block)
    let expr = alloc_string("hello");

    assert!(
        !arm_returns_converted_string(&expr),
        "Direct string literal (not in block) should return false"
    );
}

#[test]
fn test_non_block_int_literal() {
    // Just 42 (not in a block)
    let expr = alloc_int(42);

    assert!(!arm_returns_converted_string(expr));
}

// ============================================================================
// COMPLEX NESTED TESTS
// ============================================================================

#[test]
fn test_block_with_if_else_nested_string_literals() {
    // { if cond { if inner { "a" } else { "b" } } else { "c" } }
    let inner_if = alloc_stmt_if(
        alloc_int(1),
        vec![alloc_stmt_expr(alloc_string("a"))],
        Some(vec![alloc_stmt_expr(alloc_string("b"))]),
    );

    let outer_if = alloc_stmt_if(
        alloc_int(1),
        vec![inner_if],
        Some(vec![alloc_stmt_expr(alloc_string("c"))]),
    );

    let expr = alloc_block(vec![outer_if]);

    // The outer if's then branch doesn't end with a string literal expression,
    // it ends with an if statement, so this should return false
    assert!(!arm_returns_converted_string(expr));
}

#[test]
fn test_block_with_multiple_statements_last_is_string() {
    // { let x = 1; "result" }
    let let_stmt = test_alloc_stmt(Statement::Let {
        pattern: windjammer::parser::Pattern::Identifier("x".to_string()),
        mutable: false,
        type_: None,
        value: alloc_int(1),
        else_block: None,
        location: None,
    });

    let expr = alloc_block(vec![let_stmt, alloc_stmt_expr(alloc_string("result"))]);

    assert!(arm_returns_converted_string(expr));
}

#[test]
fn test_block_with_multiple_statements_last_is_int() {
    // { let x = 1; 42 }
    let let_stmt = test_alloc_stmt(Statement::Let {
        pattern: windjammer::parser::Pattern::Identifier("x".to_string()),
        mutable: false,
        type_: None,
        value: alloc_int(1),
        else_block: None,
        location: None,
    });

    let expr = alloc_block(vec![let_stmt, alloc_stmt_expr(alloc_int(42))]);

    assert!(!arm_returns_converted_string(expr));
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_block_with_if_empty_then_block() {
    // { if cond { } else { "no" } }
    let if_statement = alloc_stmt_if(
        alloc_int(1),
        vec![],
        Some(vec![alloc_stmt_expr(alloc_string("no"))]),
    );

    let expr = alloc_block(vec![if_statement]);

    assert!(!arm_returns_converted_string(expr));
}

#[test]
fn test_block_with_if_empty_else_block() {
    // { if cond { "yes" } else { } }
    let if_statement = alloc_stmt_if(
        alloc_int(1),
        vec![alloc_stmt_expr(alloc_string("yes"))],
        Some(vec![]),
    );

    let expr = alloc_block(vec![if_statement]);

    assert!(!arm_returns_converted_string(expr));
}

#[test]
fn test_block_with_return_statement() {
    // { return "value"; }
    let return_stmt = test_alloc_stmt(Statement::Return {
        value: Some(alloc_string("value")),
        location: None,
    });

    let expr = alloc_block(vec![return_stmt]);

    assert!(
        !arm_returns_converted_string(&expr),
        "Return statement should return false"
    );
}
