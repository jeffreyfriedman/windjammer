// TDD Tests for expression helper functions (Phase 8)
// These tests are written BEFORE extracting the functions

use std::path::PathBuf;
use windjammer::codegen::rust::expression_helpers::{is_const_evaluable, is_reference_expression};
use windjammer::parser::{BinaryOp, Expression, Literal, UnaryOp};
use windjammer::source_map::Location;

fn test_loc() -> Location {
    Location {
        file: PathBuf::from(""),
        line: 0,
        column: 0,
    }
}

// =============================================================================
// is_reference_expression Tests
// =============================================================================

#[test]
fn test_is_reference_expression_ref() {
    // &x → true
    let expr = Expression::Unary {
        op: UnaryOp::Ref,
        operand: Box::new(Expression::Identifier {
            name: "x".to_string(),
            location: Some(test_loc()),
        }),
        location: Some(test_loc()),
    };
    assert!(is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_mut_ref() {
    // &mut x → true
    let expr = Expression::Unary {
        op: UnaryOp::MutRef,
        operand: Box::new(Expression::Identifier {
            name: "x".to_string(),
            location: Some(test_loc()),
        }),
        location: Some(test_loc()),
    };
    assert!(is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_not() {
    // !x → false
    let expr = Expression::Unary {
        op: UnaryOp::Not,
        operand: Box::new(Expression::Identifier {
            name: "x".to_string(),
            location: Some(test_loc()),
        }),
        location: Some(test_loc()),
    };
    assert!(!is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_identifier() {
    // x → false
    let expr = Expression::Identifier {
        name: "x".to_string(),
        location: Some(test_loc()),
    };
    assert!(!is_reference_expression(&expr));
}

#[test]
fn test_is_reference_expression_literal() {
    // 42 → false
    let expr = Expression::Literal {
        value: Literal::Int(42),
        location: Some(test_loc()),
    };
    assert!(!is_reference_expression(&expr));
}

// =============================================================================
// is_const_evaluable Tests
// =============================================================================

#[test]
fn test_is_const_evaluable_literal_int() {
    // 42 → true
    let expr = Expression::Literal {
        value: Literal::Int(42),
        location: Some(test_loc()),
    };
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_literal_string() {
    // "hello" → true
    let expr = Expression::Literal {
        value: Literal::String("hello".to_string()),
        location: Some(test_loc()),
    };
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_literal_bool() {
    // true → true
    let expr = Expression::Literal {
        value: Literal::Bool(true),
        location: Some(test_loc()),
    };
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_binary_const() {
    // 1 + 2 → true
    let expr = Expression::Binary {
        left: Box::new(Expression::Literal {
            value: Literal::Int(1),
            location: Some(test_loc()),
        }),
        op: BinaryOp::Add,
        right: Box::new(Expression::Literal {
            value: Literal::Int(2),
            location: Some(test_loc()),
        }),
        location: Some(test_loc()),
    };
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_binary_non_const() {
    // x + 2 → false
    let expr = Expression::Binary {
        left: Box::new(Expression::Identifier {
            name: "x".to_string(),
            location: Some(test_loc()),
        }),
        op: BinaryOp::Add,
        right: Box::new(Expression::Literal {
            value: Literal::Int(2),
            location: Some(test_loc()),
        }),
        location: Some(test_loc()),
    };
    assert!(!is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_unary_const() {
    // -5 → true
    let expr = Expression::Unary {
        op: UnaryOp::Neg,
        operand: Box::new(Expression::Literal {
            value: Literal::Int(5),
            location: Some(test_loc()),
        }),
        location: Some(test_loc()),
    };
    assert!(is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_unary_non_const() {
    // -x → false
    let expr = Expression::Unary {
        op: UnaryOp::Neg,
        operand: Box::new(Expression::Identifier {
            name: "x".to_string(),
            location: Some(test_loc()),
        }),
        location: Some(test_loc()),
    };
    assert!(!is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_identifier() {
    // x → false
    let expr = Expression::Identifier {
        name: "x".to_string(),
        location: Some(test_loc()),
    };
    assert!(!is_const_evaluable(&expr));
}

#[test]
fn test_is_const_evaluable_complex() {
    // (1 + 2) * 3 → true
    let expr = Expression::Binary {
        left: Box::new(Expression::Binary {
            left: Box::new(Expression::Literal {
                value: Literal::Int(1),
                location: Some(test_loc()),
            }),
            op: BinaryOp::Add,
            right: Box::new(Expression::Literal {
                value: Literal::Int(2),
                location: Some(test_loc()),
            }),
            location: Some(test_loc()),
        }),
        op: BinaryOp::Mul,
        right: Box::new(Expression::Literal {
            value: Literal::Int(3),
            location: Some(test_loc()),
        }),
        location: Some(test_loc()),
    };
    assert!(is_const_evaluable(&expr));
}
