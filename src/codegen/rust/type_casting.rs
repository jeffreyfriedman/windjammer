// Auto-casting utilities for type conversions
// Extracted from generator.rs for better modularity

use crate::parser::ast::*;

/// Check if an expression produces a usize result
pub fn expression_produces_usize(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::MethodCall {
            method,
            ..
        } if method == "len"
    )
}

/// Check if an expression is a usize literal
pub fn is_usize_literal(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Literal {
            value: Literal::Int(_),
            ..
        }
    )
}

/// Generate a cast from usize to i64 if needed
pub fn maybe_cast_usize_to_int(expr_str: String, needs_cast: bool) -> String {
    if needs_cast {
        format!("({} as i64)", expr_str)
    } else {
        expr_str
    }
}

/// Generate a cast for usize in binary operations
pub fn cast_for_usize_binary_op(left_str: &str, right_str: &str, left_is_usize: bool, right_is_usize: bool) -> (String, String) {
    match (left_is_usize, right_is_usize) {
        (true, false) => {
            // Cast left (usize) to match right (int)
            (format!("({} as i64)", left_str), right_str.to_string())
        }
        (false, true) => {
            // Cast right (usize) to match left (int)
            (left_str.to_string(), format!("({} as i64)", right_str))
        }
        _ => {
            // No casting needed
            (left_str.to_string(), right_str.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_produces_usize() {
        let expr = Expression::MethodCall {
            object: Box::new(Expression::Identifier {
                name: "vec".to_string(),
                location: None,
            }),
            method: "len".to_string(),
            type_args: None,
            arguments: vec![],
            location: None,
        };
        assert!(expression_produces_usize(&expr));
    }

    #[test]
    fn test_maybe_cast_usize_to_int() {
        assert_eq!(maybe_cast_usize_to_int("vec.len()".to_string(), true), "(vec.len() as i64)");
        assert_eq!(maybe_cast_usize_to_int("42".to_string(), false), "42");
    }

    #[test]
    fn test_cast_for_usize_binary_op() {
        let (left, right) = cast_for_usize_binary_op("vec.len()", "10", true, false);
        assert_eq!(left, "(vec.len() as i64)");
        assert_eq!(right, "10");

        let (left, right) = cast_for_usize_binary_op("10", "vec.len()", false, true);
        assert_eq!(left, "10");
        assert_eq!(right, "(vec.len() as i64)");

        let (left, right) = cast_for_usize_binary_op("x", "y", false, false);
        assert_eq!(left, "x");
        assert_eq!(right, "y");
    }
}

