// TDD Tests for Match Arm String Analysis
// Tests written FIRST before extraction!

use windjammer::codegen::rust::arm_string_analysis::*;
use windjammer::parser::{Expression, Literal, Statement};

// Helper to create string literal expression
fn string_lit(s: &str) -> Expression {
    Expression::Literal {
        value: Literal::String(s.to_string()),
        location: None,
    }
}

// Helper to create int literal expression
fn int_lit(n: i64) -> Expression {
    Expression::Literal {
        value: Literal::Int(n),
        location: None,
    }
}

// Helper to create block expression
fn block_expr(statements: Vec<Statement>) -> Expression {
    Expression::Block {
        statements,
        location: None,
    }
}

// Helper to create expression statement
fn expr_stmt(expr: Expression) -> Statement {
    Statement::Expression {
        expr,
        location: None,
    }
}

// Helper to create if statement
fn if_stmt(
    condition: Expression,
    then_block: Vec<Statement>,
    else_block: Option<Vec<Statement>>,
) -> Statement {
    Statement::If {
        condition,
        then_block,
        else_block,
        location: None,
    }
}

// ============================================================================
// BLOCK WITH IF-ELSE TESTS
// ============================================================================

#[test]
fn test_block_with_if_else_both_string_literals() {
    // { if cond { "yes" } else { "no" } }
    let if_statement = if_stmt(
        int_lit(1), // condition (doesn't matter for this test)
        vec![expr_stmt(string_lit("yes"))],
        Some(vec![expr_stmt(string_lit("no"))]),
    );

    let expr = block_expr(vec![if_statement]);

    assert!(arm_returns_converted_string(&expr));
}

#[test]
fn test_block_with_if_else_then_string_else_int() {
    // { if cond { "yes" } else { 42 } }
    let if_statement = if_stmt(
        int_lit(1),
        vec![expr_stmt(string_lit("yes"))],
        Some(vec![expr_stmt(int_lit(42))]),
    );

    let expr = block_expr(vec![if_statement]);

    assert!(!arm_returns_converted_string(&expr));
}

#[test]
fn test_block_with_if_else_then_int_else_string() {
    // { if cond { 42 } else { "no" } }
    let if_statement = if_stmt(
        int_lit(1),
        vec![expr_stmt(int_lit(42))],
        Some(vec![expr_stmt(string_lit("no"))]),
    );

    let expr = block_expr(vec![if_statement]);

    assert!(!arm_returns_converted_string(&expr));
}

#[test]
fn test_block_with_if_no_else() {
    // { if cond { "yes" } }
    let if_statement = if_stmt(int_lit(1), vec![expr_stmt(string_lit("yes"))], None);

    let expr = block_expr(vec![if_statement]);

    assert!(
        !arm_returns_converted_string(&expr),
        "If without else should return false"
    );
}

#[test]
fn test_block_with_if_else_both_ints() {
    // { if cond { 1 } else { 2 } }
    let if_statement = if_stmt(
        int_lit(1),
        vec![expr_stmt(int_lit(1))],
        Some(vec![expr_stmt(int_lit(2))]),
    );

    let expr = block_expr(vec![if_statement]);

    assert!(!arm_returns_converted_string(&expr));
}

// ============================================================================
// BLOCK WITH EXPRESSION STATEMENT TESTS
// ============================================================================

#[test]
fn test_block_with_string_literal_expression() {
    // { "hello" }
    let expr = block_expr(vec![expr_stmt(string_lit("hello"))]);

    assert!(arm_returns_converted_string(&expr));
}

#[test]
fn test_block_with_int_literal_expression() {
    // { 42 }
    let expr = block_expr(vec![expr_stmt(int_lit(42))]);

    assert!(!arm_returns_converted_string(&expr));
}

#[test]
fn test_block_with_nested_block_string_literal() {
    // { { "nested" } }
    let inner_block = block_expr(vec![expr_stmt(string_lit("nested"))]);
    let outer_block = block_expr(vec![expr_stmt(inner_block)]);

    assert!(arm_returns_converted_string(&outer_block));
}

#[test]
fn test_block_with_nested_block_int_literal() {
    // { { 42 } }
    let inner_block = block_expr(vec![expr_stmt(int_lit(42))]);
    let outer_block = block_expr(vec![expr_stmt(inner_block)]);

    assert!(!arm_returns_converted_string(&outer_block));
}

// ============================================================================
// EMPTY BLOCK TESTS
// ============================================================================

#[test]
fn test_empty_block() {
    // { }
    let expr = block_expr(vec![]);

    assert!(!arm_returns_converted_string(&expr));
}

// ============================================================================
// NON-BLOCK EXPRESSION TESTS
// ============================================================================

#[test]
fn test_non_block_string_literal() {
    // Just "hello" (not in a block)
    let expr = string_lit("hello");

    assert!(
        !arm_returns_converted_string(&expr),
        "Direct string literal (not in block) should return false"
    );
}

#[test]
fn test_non_block_int_literal() {
    // Just 42 (not in a block)
    let expr = int_lit(42);

    assert!(!arm_returns_converted_string(&expr));
}

// ============================================================================
// COMPLEX NESTED TESTS
// ============================================================================

#[test]
fn test_block_with_if_else_nested_string_literals() {
    // { if cond { if inner { "a" } else { "b" } } else { "c" } }
    let inner_if = if_stmt(
        int_lit(1),
        vec![expr_stmt(string_lit("a"))],
        Some(vec![expr_stmt(string_lit("b"))]),
    );

    let outer_if = if_stmt(
        int_lit(1),
        vec![inner_if],
        Some(vec![expr_stmt(string_lit("c"))]),
    );

    let expr = block_expr(vec![outer_if]);

    // The outer if's then branch doesn't end with a string literal expression,
    // it ends with an if statement, so this should return false
    assert!(!arm_returns_converted_string(&expr));
}

#[test]
fn test_block_with_multiple_statements_last_is_string() {
    // { let x = 1; "result" }
    let let_stmt = Statement::Let {
        pattern: windjammer::parser::Pattern::Identifier("x".to_string()),
        mutable: false,
        type_: None,
        value: int_lit(1),
        else_block: None,
        location: None,
    };

    let expr = block_expr(vec![let_stmt, expr_stmt(string_lit("result"))]);

    assert!(arm_returns_converted_string(&expr));
}

#[test]
fn test_block_with_multiple_statements_last_is_int() {
    // { let x = 1; 42 }
    let let_stmt = Statement::Let {
        pattern: windjammer::parser::Pattern::Identifier("x".to_string()),
        mutable: false,
        type_: None,
        value: int_lit(1),
        else_block: None,
        location: None,
    };

    let expr = block_expr(vec![let_stmt, expr_stmt(int_lit(42))]);

    assert!(!arm_returns_converted_string(&expr));
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_block_with_if_empty_then_block() {
    // { if cond { } else { "no" } }
    let if_statement = if_stmt(int_lit(1), vec![], Some(vec![expr_stmt(string_lit("no"))]));

    let expr = block_expr(vec![if_statement]);

    assert!(!arm_returns_converted_string(&expr));
}

#[test]
fn test_block_with_if_empty_else_block() {
    // { if cond { "yes" } else { } }
    let if_statement = if_stmt(int_lit(1), vec![expr_stmt(string_lit("yes"))], Some(vec![]));

    let expr = block_expr(vec![if_statement]);

    assert!(!arm_returns_converted_string(&expr));
}

#[test]
fn test_block_with_return_statement() {
    // { return "value"; }
    let return_stmt = Statement::Return {
        value: Some(string_lit("value")),
        location: None,
    };

    let expr = block_expr(vec![return_stmt]);

    assert!(
        !arm_returns_converted_string(&expr),
        "Return statement should return false"
    );
}
