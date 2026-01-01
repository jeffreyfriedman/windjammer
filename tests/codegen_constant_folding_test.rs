// TDD Tests for Constant Folding Optimization
// Tests written FIRST before extraction!
//
// UPDATED: Now using standard AST builder functions!

use windjammer::codegen::rust::constant_folding::*;
use windjammer::parser::{BinaryOp, Expression, Literal, UnaryOp};
use windjammer::test_utils::*;

// Re-alias builders for backward compatibility with existing tests
fn int_lit(n: i64) -> &'static Expression<'static> {
    test_alloc_expr(Expression::Literal {
        value: Literal::Int(n),
        location: None,
    })
}

fn float_lit(f: f64) -> &'static Expression<'static> {
    test_alloc_expr(Expression::Literal {
        value: Literal::Float(f),
        location: None,
    })
}

fn bool_lit(b: bool) -> &'static Expression<'static> {
    test_alloc_expr(Expression::Literal {
        value: Literal::Bool(b),
        location: None,
    })
}

fn binary(left: &'static Expression<'static>, op: BinaryOp, right: &'static Expression<'static>) -> &'static Expression<'static> {
    test_alloc_expr(Expression::Binary {
        left,
        op,
        right,
        location: None,
    })
}

fn unary(op: UnaryOp, operand: &'static Expression<'static>) -> &'static Expression<'static> {
    test_alloc_expr(Expression::Unary {
        op,
        operand,
        location: None,
    })
}

// ============================================================================
// INTEGER ARITHMETIC TESTS
// ============================================================================

#[test]
fn test_fold_int_add() {
    let expr = binary(int_lit(5), BinaryOp::Add, int_lit(3));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, 8),
        _ => panic!("Expected Int literal"),
    }
}

#[test]
fn test_fold_int_sub() {
    let expr = binary(int_lit(10), BinaryOp::Sub, int_lit(4));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, 6),
        _ => panic!("Expected Int literal"),
    }
}

#[test]
fn test_fold_int_mul() {
    let expr = binary(int_lit(6), BinaryOp::Mul, int_lit(7));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, 42),
        _ => panic!("Expected Int literal"),
    }
}

#[test]
fn test_fold_int_div() {
    let expr = binary(int_lit(20), BinaryOp::Div, int_lit(4));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, 5),
        _ => panic!("Expected Int literal"),
    }
}

#[test]
fn test_fold_int_mod() {
    let expr = binary(int_lit(17), BinaryOp::Mod, int_lit(5));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, 2),
        _ => panic!("Expected Int literal"),
    }
}

#[test]
fn test_no_fold_int_div_by_zero() {
    let expr = binary(int_lit(10), BinaryOp::Div, int_lit(0));
    let result = try_fold_constant(&expr);
    assert!(result.is_none(), "Should not fold division by zero");
}

#[test]
fn test_no_fold_int_mod_by_zero() {
    let expr = binary(int_lit(10), BinaryOp::Mod, int_lit(0));
    let result = try_fold_constant(&expr);
    assert!(result.is_none(), "Should not fold modulo by zero");
}

// ============================================================================
// FLOAT ARITHMETIC TESTS
// ============================================================================

#[test]
fn test_fold_float_add() {
    let expr = binary(float_lit(3.5), BinaryOp::Add, float_lit(2.5));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Float(f),
            ..
        } => assert!((f - 6.0).abs() < 0.0001),
        _ => panic!("Expected Float literal"),
    }
}

#[test]
fn test_fold_float_sub() {
    let expr = binary(float_lit(10.5), BinaryOp::Sub, float_lit(3.5));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Float(f),
            ..
        } => assert!((f - 7.0).abs() < 0.0001),
        _ => panic!("Expected Float literal"),
    }
}

#[test]
fn test_fold_float_mul() {
    let expr = binary(float_lit(2.5), BinaryOp::Mul, float_lit(4.0));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Float(f),
            ..
        } => assert!((f - 10.0).abs() < 0.0001),
        _ => panic!("Expected Float literal"),
    }
}

#[test]
fn test_fold_float_div() {
    let expr = binary(float_lit(15.0), BinaryOp::Div, float_lit(3.0));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Float(f),
            ..
        } => assert!((f - 5.0).abs() < 0.0001),
        _ => panic!("Expected Float literal"),
    }
}

#[test]
fn test_no_fold_float_div_by_zero() {
    let expr = binary(float_lit(10.0), BinaryOp::Div, float_lit(0.0));
    let result = try_fold_constant(&expr);
    assert!(result.is_none(), "Should not fold float division by zero");
}

// ============================================================================
// INTEGER COMPARISON TESTS
// ============================================================================

#[test]
fn test_fold_int_eq_true() {
    let expr = binary(int_lit(5), BinaryOp::Eq, int_lit(5));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_int_eq_false() {
    let expr = binary(int_lit(5), BinaryOp::Eq, int_lit(3));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(!b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_int_ne() {
    let expr = binary(int_lit(5), BinaryOp::Ne, int_lit(3));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_int_lt() {
    let expr = binary(int_lit(3), BinaryOp::Lt, int_lit(5));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_int_le() {
    let expr = binary(int_lit(5), BinaryOp::Le, int_lit(5));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_int_gt() {
    let expr = binary(int_lit(7), BinaryOp::Gt, int_lit(5));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_int_ge() {
    let expr = binary(int_lit(5), BinaryOp::Ge, int_lit(5));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(b),
        _ => panic!("Expected Bool literal"),
    }
}

// ============================================================================
// BOOLEAN OPERATION TESTS
// ============================================================================

#[test]
fn test_fold_bool_and_true() {
    let expr = binary(bool_lit(true), BinaryOp::And, bool_lit(true));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_bool_and_false() {
    let expr = binary(bool_lit(true), BinaryOp::And, bool_lit(false));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(!b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_bool_or_true() {
    let expr = binary(bool_lit(false), BinaryOp::Or, bool_lit(true));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_bool_or_false() {
    let expr = binary(bool_lit(false), BinaryOp::Or, bool_lit(false));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(!b),
        _ => panic!("Expected Bool literal"),
    }
}

// ============================================================================
// UNARY OPERATION TESTS
// ============================================================================

#[test]
fn test_fold_unary_neg_int() {
    let expr = unary(UnaryOp::Neg, int_lit(42));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, -42),
        _ => panic!("Expected Int literal"),
    }
}

#[test]
fn test_fold_unary_neg_float() {
    let expr = unary(UnaryOp::Neg, float_lit(2.5));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Float(f),
            ..
        } => assert!((f + 2.5).abs() < 0.0001),
        _ => panic!("Expected Float literal"),
    }
}

#[test]
fn test_fold_unary_not_true() {
    let expr = unary(UnaryOp::Not, bool_lit(true));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(!b),
        _ => panic!("Expected Bool literal"),
    }
}

#[test]
fn test_fold_unary_not_false() {
    let expr = unary(UnaryOp::Not, bool_lit(false));
    let result = try_fold_constant(&expr);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Bool(b),
            ..
        } => assert!(b),
        _ => panic!("Expected Bool literal"),
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_no_fold_already_literal() {
    let expr = int_lit(42);
    let result = try_fold_constant(&expr);
    assert!(result.is_none(), "Literal cannot be folded further");
}

#[test]
fn test_no_fold_non_constant() {
    let expr = Expression::Identifier {
        name: "x".to_string(),
        location: None,
    };
    let result = try_fold_constant(&expr);
    assert!(result.is_none(), "Non-constant cannot be folded");
}

#[test]
fn test_no_fold_mixed_types() {
    // Int + Float should not fold (type mismatch)
    let expr = binary(int_lit(5), BinaryOp::Add, float_lit(3.5));
    let result = try_fold_constant(&expr);
    assert!(result.is_none(), "Mixed types should not fold");
}

// ============================================================================
// RECURSIVE FOLDING TESTS
// ============================================================================

#[test]
fn test_fold_nested_add() {
    // (2 + 3) + 4 should fold to 9
    let inner = binary(int_lit(2), BinaryOp::Add, int_lit(3));
    let outer = binary(inner, BinaryOp::Add, int_lit(4));
    let result = try_fold_constant(&outer);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, 9),
        _ => panic!("Expected Int literal"),
    }
}

#[test]
fn test_fold_nested_mul_add() {
    // (2 * 3) + 4 should fold to 10
    let inner = binary(int_lit(2), BinaryOp::Mul, int_lit(3));
    let outer = binary(inner, BinaryOp::Add, int_lit(4));
    let result = try_fold_constant(&outer);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, 10),
        _ => panic!("Expected Int literal"),
    }
}

#[test]
fn test_fold_double_negation() {
    // --42 should fold to 42
    let inner = unary(UnaryOp::Neg, int_lit(42));
    let outer = unary(UnaryOp::Neg, inner);
    let result = try_fold_constant(&outer);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, 42),
        _ => panic!("Expected Int literal"),
    }
}

#[test]
fn test_fold_complex_expression() {
    // ((5 + 3) * 2) - 4 should fold to 12
    let add = binary(int_lit(5), BinaryOp::Add, int_lit(3));
    let mul = binary(add, BinaryOp::Mul, int_lit(2));
    let sub = binary(mul, BinaryOp::Sub, int_lit(4));
    let result = try_fold_constant(&sub);
    assert!(result.is_some());
    match result.unwrap() {
        Expression::Literal {
            value: Literal::Int(n),
            ..
        } => assert_eq!(n, 12),
        _ => panic!("Expected Int literal"),
    }
}
