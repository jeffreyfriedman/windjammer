// TDD Tests for string expression analysis functions
//
// This module contains pure functions for analyzing string-related expressions:
// - Collecting string concatenation parts
// - Detecting string literals in expressions
//
// UPDATED: Now using AST builder functions for cleaner, more readable tests!

use windjammer::codegen::rust::string_analysis::{collect_concat_parts, contains_string_literal};
use windjammer::parser::ast::builders::*;
use windjammer::parser::{BinaryOp, Expression, Literal};

#[cfg(test)]
mod collect_concat_parts_tests {
    use super::*;

    #[test]
    fn test_single_string_literal() {
        // Test: "hello" → ["hello"]
        let expr = expr_string("hello");

        let parts = collect_concat_parts(&expr);
        assert_eq!(parts.len(), 1);

        match &parts[0] {
            Expression::Literal {
                value: Literal::String(s),
                ..
            } => {
                assert_eq!(s, "hello");
            }
            _ => panic!("Expected string literal"),
        }
    }

    #[test]
    fn test_two_string_concat() {
        // Test: "hello" + "world" → ["hello", "world"]
        let expr = expr_add(expr_string("hello"), expr_string("world"));

        let parts = collect_concat_parts(&expr);
        assert_eq!(parts.len(), 2);

        match (&parts[0], &parts[1]) {
            (
                Expression::Literal {
                    value: Literal::String(s1),
                    ..
                },
                Expression::Literal {
                    value: Literal::String(s2),
                    ..
                },
            ) => {
                assert_eq!(s1, "hello");
                assert_eq!(s2, "world");
            }
            _ => panic!("Expected two string literals"),
        }
    }

    #[test]
    fn test_three_string_concat_chain() {
        // Test: "a" + "b" + "c" → ["a", "b", "c"]
        // ("a" + "b") + "c"
        let abc = expr_add(
            expr_add(expr_string("a"), expr_string("b")),
            expr_string("c")
        );

        let parts = collect_concat_parts(&abc);
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_mixed_expression_concat() {
        // Test: "hello" + variable_name → ["hello", variable_name]
        let expr = expr_add(expr_string("hello"), expr_var("name"));

        let parts = collect_concat_parts(&expr);
        assert_eq!(parts.len(), 2);

        match (&parts[0], &parts[1]) {
            (
                Expression::Literal {
                    value: Literal::String(s),
                    ..
                },
                Expression::Identifier { name, .. },
            ) => {
                assert_eq!(s, "hello");
                assert_eq!(name, "name");
            }
            _ => panic!("Expected string literal and identifier"),
        }
    }

    #[test]
    fn test_non_add_binary_expression() {
        // Test: a * b → [a * b] (not a concatenation)
        let expr = expr_mul(expr_var("a"), expr_var("b"));

        let parts = collect_concat_parts(&expr);
        assert_eq!(parts.len(), 1); // Whole expression, not split

        match &parts[0] {
            Expression::Binary {
                op: BinaryOp::Mul, ..
            } => {
                // Correct: preserved as multiplication
            }
            _ => panic!("Expected multiplication to be preserved"),
        }
    }
}

#[cfg(test)]
mod contains_string_literal_tests {
    use super::*;

    #[test]
    fn test_single_string_literal() {
        // Test: "hello" → true
        let expr = expr_string("hello");
        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_number_literal() {
        // Test: 42 → false
        let expr = expr_int(42);
        assert!(!contains_string_literal(&expr));
    }

    #[test]
    fn test_identifier() {
        // Test: variable_name → false
        let expr = expr_var("variable_name");
        assert!(!contains_string_literal(&expr));
    }

    #[test]
    fn test_binary_with_string_left() {
        // Test: "hello" + variable → true
        let expr = expr_add(expr_string("hello"), expr_var("name"));
        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_binary_with_string_right() {
        // Test: variable + "world" → true
        let expr = expr_add(expr_var("name"), expr_string("world"));
        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_binary_no_strings() {
        // Test: a + b → false
        let expr = expr_add(expr_var("a"), expr_var("b"));
        assert!(!contains_string_literal(&expr));
    }

    #[test]
    fn test_nested_binary_with_string() {
        // Test: (a + b) + "hello" → true
        let expr = expr_add(
            expr_add(expr_var("a"), expr_var("b")),
            expr_string("hello")
        );
        assert!(contains_string_literal(&expr));
    }
}
