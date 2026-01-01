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
use windjammer::test_utils::*;

// Helper wrappers for arena allocation
fn alloc_string(s: &str) -> &'static Expression<'static> {
    test_alloc_expr(expr_string(s))
}

fn alloc_var(name: &str) -> &'static Expression<'static> {
    test_alloc_expr(expr_var(name))
}

fn alloc_add(left: &'static Expression<'static>, right: &'static Expression<'static>) -> &'static Expression<'static> {
    test_alloc_expr(expr_add(left, right))
}

fn alloc_mul(left: &'static Expression<'static>, right: &'static Expression<'static>) -> &'static Expression<'static> {
    test_alloc_expr(expr_binary(BinaryOp::Mul, left, right))
}

#[cfg(test)]
mod collect_concat_parts_tests {
    use super::*;

    #[test]
    fn test_single_string_literal() {
        // Test: "hello" → ["hello"]
        let expr = alloc_string("hello");

        let parts = collect_concat_parts(expr);
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
        let expr = alloc_add(alloc_string("hello"), alloc_string("world"));

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
        let abc = alloc_add(
            alloc_add(alloc_string("a"), alloc_string("b")),
            alloc_string("c"),
        );

        let parts = collect_concat_parts(&abc);
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_mixed_expression_concat() {
        // Test: "hello" + variable_name → ["hello", variable_name]
        let expr = alloc_add(alloc_string("hello"), alloc_var("name"));

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
        let expr = alloc_mul(alloc_var("a"), alloc_var("b"));

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
        let expr = alloc_string("hello");
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
        let expr = alloc_var("variable_name");
        assert!(!contains_string_literal(&expr));
    }

    #[test]
    fn test_binary_with_string_left() {
        // Test: "hello" + variable → true
        let expr = alloc_add(alloc_string("hello"), alloc_var("name"));
        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_binary_with_string_right() {
        // Test: variable + "world" → true
        let expr = alloc_add(alloc_var("name"), alloc_string("world"));
        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_binary_no_strings() {
        // Test: a + b → false
        let expr = alloc_add(alloc_var("a"), alloc_var("b"));
        assert!(!contains_string_literal(&expr));
    }

    #[test]
    fn test_nested_binary_with_string() {
        // Test: (a + b) + "hello" → true
        let expr = alloc_add(alloc_add(alloc_var("a"), alloc_var("b")), alloc_string("hello"));
        assert!(contains_string_literal(&expr));
    }
}
