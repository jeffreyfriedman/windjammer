// String expression analysis utilities
//
// This module provides functions for analyzing string-related expressions:
// - Collecting string concatenation parts
// - Detecting string literals in expressions

use crate::parser::{BinaryOp, Expression, Literal};

/// Collects all parts of a string concatenation chain
///
/// For expressions like `"a" + "b" + "c"`, this returns `["a", "b", "c"]`.
/// For non-concatenation expressions, returns the expression itself as a single element.
///
/// # Examples
/// ```
/// // "hello" + "world" → ["hello", "world"]
/// // "a" + variable → ["a", variable]
/// // a * b → [a * b] (not a concatenation)
/// ```
pub fn collect_concat_parts(expr: &Expression) -> Vec<Expression> {
    let mut parts = Vec::new();
    collect_concat_parts_recursive(expr, &mut parts);
    parts
}

/// Recursively collect string concatenation parts
fn collect_concat_parts_recursive(expr: &Expression, parts: &mut Vec<Expression>) {
    match expr {
        Expression::Binary {
            left,
            op: BinaryOp::Add,
            right,
            ..
        } => {
            // Recursively collect parts from both sides of the + operator
            collect_concat_parts_recursive(left, parts);
            collect_concat_parts_recursive(right, parts);
        }
        _ => {
            // Not an addition, treat as a single part
            parts.push(expr.clone());
        }
    }
}

/// Checks if an expression contains a string literal (recursively)
///
/// This is useful for detecting string operations that might need special handling.
///
/// # Examples
/// ```
/// // "hello" → true
/// // 42 → false
/// // "hello" + variable → true
/// // variable + "world" → true
/// // a + b → false
/// ```
pub fn contains_string_literal(expr: &Expression) -> bool {
    match expr {
        Expression::Literal {
            value: Literal::String(_),
            ..
        } => true,
        Expression::Binary { left, right, .. } => {
            // Recursively check both sides
            contains_string_literal(left) || contains_string_literal(right)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_map::Location;
    use std::path::PathBuf;

    fn test_loc() -> Location {
        Location {
            file: PathBuf::from(""),
            line: 0,
            column: 0,
        }
    }

    #[test]
    fn test_collect_single_expression() {
        let expr = Expression::Identifier {
            name: "x".to_string(),
            location: Some(test_loc()),
        };

        let parts = collect_concat_parts(&expr);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_collect_nested_concatenation() {
        // ("a" + "b") + ("c" + "d")
        let a = Expression::Literal {
            value: Literal::String("a".to_string()),
            location: Some(test_loc()),
        };
        let b = Expression::Literal {
            value: Literal::String("b".to_string()),
            location: Some(test_loc()),
        };
        let c = Expression::Literal {
            value: Literal::String("c".to_string()),
            location: Some(test_loc()),
        };
        let d = Expression::Literal {
            value: Literal::String("d".to_string()),
            location: Some(test_loc()),
        };

        let ab = Expression::Binary {
            left: Box::new(a),
            op: BinaryOp::Add,
            right: Box::new(b),
            location: Some(test_loc()),
        };
        let cd = Expression::Binary {
            left: Box::new(c),
            op: BinaryOp::Add,
            right: Box::new(d),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(ab),
            op: BinaryOp::Add,
            right: Box::new(cd),
            location: Some(test_loc()),
        };

        let parts = collect_concat_parts(&expr);
        assert_eq!(parts.len(), 4); // Should flatten to ["a", "b", "c", "d"]
    }

    #[test]
    fn test_contains_string_in_nested_expression() {
        // ((a + b) * c) + "hello"
        let a = Expression::Identifier {
            name: "a".to_string(),
            location: Some(test_loc()),
        };
        let b = Expression::Identifier {
            name: "b".to_string(),
            location: Some(test_loc()),
        };
        let c = Expression::Identifier {
            name: "c".to_string(),
            location: Some(test_loc()),
        };
        let hello = Expression::Literal {
            value: Literal::String("hello".to_string()),
            location: Some(test_loc()),
        };

        let ab = Expression::Binary {
            left: Box::new(a),
            op: BinaryOp::Add,
            right: Box::new(b),
            location: Some(test_loc()),
        };
        let ab_mul_c = Expression::Binary {
            left: Box::new(ab),
            op: BinaryOp::Mul,
            right: Box::new(c),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(ab_mul_c),
            op: BinaryOp::Add,
            right: Box::new(hello),
            location: Some(test_loc()),
        };

        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_no_string_in_complex_expression() {
        // (a + b) * (c - d)
        let a = Expression::Identifier {
            name: "a".to_string(),
            location: Some(test_loc()),
        };
        let b = Expression::Identifier {
            name: "b".to_string(),
            location: Some(test_loc()),
        };
        let c = Expression::Identifier {
            name: "c".to_string(),
            location: Some(test_loc()),
        };
        let d = Expression::Identifier {
            name: "d".to_string(),
            location: Some(test_loc()),
        };

        let ab = Expression::Binary {
            left: Box::new(a),
            op: BinaryOp::Add,
            right: Box::new(b),
            location: Some(test_loc()),
        };
        let cd = Expression::Binary {
            left: Box::new(c),
            op: BinaryOp::Sub,
            right: Box::new(d),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(ab),
            op: BinaryOp::Mul,
            right: Box::new(cd),
            location: Some(test_loc()),
        };

        assert!(!contains_string_literal(&expr));
    }
}
