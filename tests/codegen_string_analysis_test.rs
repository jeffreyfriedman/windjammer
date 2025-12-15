// TDD Tests for string expression analysis functions
//
// This module contains pure functions for analyzing string-related expressions:
// - Collecting string concatenation parts
// - Detecting string literals in expressions

use std::path::PathBuf;
use windjammer::codegen::rust::string_analysis::{collect_concat_parts, contains_string_literal};
use windjammer::parser::{BinaryOp, Expression, Literal};
use windjammer::source_map::Location;

// Helper to create a location for tests
fn test_loc() -> Location {
    Location {
        file: PathBuf::from(""),
        line: 0,
        column: 0,
    }
}

#[cfg(test)]
mod collect_concat_parts_tests {
    use super::*;

    #[test]
    fn test_single_string_literal() {
        // Test: "hello" → ["hello"]
        let expr = Expression::Literal {
            value: Literal::String("hello".to_string()),
            location: Some(test_loc()),
        };

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
        let left = Expression::Literal {
            value: Literal::String("hello".to_string()),
            location: Some(test_loc()),
        };
        let right = Expression::Literal {
            value: Literal::String("world".to_string()),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Add,
            right: Box::new(right),
            location: Some(test_loc()),
        };

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

        // ("a" + "b") + "c"
        let ab = Expression::Binary {
            left: Box::new(a),
            op: BinaryOp::Add,
            right: Box::new(b),
            location: Some(test_loc()),
        };
        let abc = Expression::Binary {
            left: Box::new(ab),
            op: BinaryOp::Add,
            right: Box::new(c),
            location: Some(test_loc()),
        };

        let parts = collect_concat_parts(&abc);
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_mixed_expression_concat() {
        // Test: "hello" + variable_name → ["hello", variable_name]
        let lit = Expression::Literal {
            value: Literal::String("hello".to_string()),
            location: Some(test_loc()),
        };
        let var = Expression::Identifier {
            name: "name".to_string(),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(lit),
            op: BinaryOp::Add,
            right: Box::new(var),
            location: Some(test_loc()),
        };

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
        let a = Expression::Identifier {
            name: "a".to_string(),
            location: Some(test_loc()),
        };
        let b = Expression::Identifier {
            name: "b".to_string(),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(a),
            op: BinaryOp::Mul,
            right: Box::new(b),
            location: Some(test_loc()),
        };

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
        let expr = Expression::Literal {
            value: Literal::String("hello".to_string()),
            location: Some(test_loc()),
        };

        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_number_literal() {
        // Test: 42 → false
        let expr = Expression::Literal {
            value: Literal::Int(42),
            location: Some(test_loc()),
        };

        assert!(!contains_string_literal(&expr));
    }

    #[test]
    fn test_identifier() {
        // Test: variable_name → false
        let expr = Expression::Identifier {
            name: "variable_name".to_string(),
            location: Some(test_loc()),
        };

        assert!(!contains_string_literal(&expr));
    }

    #[test]
    fn test_binary_with_string_left() {
        // Test: "hello" + variable → true
        let left = Expression::Literal {
            value: Literal::String("hello".to_string()),
            location: Some(test_loc()),
        };
        let right = Expression::Identifier {
            name: "name".to_string(),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Add,
            right: Box::new(right),
            location: Some(test_loc()),
        };

        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_binary_with_string_right() {
        // Test: variable + "world" → true
        let left = Expression::Identifier {
            name: "name".to_string(),
            location: Some(test_loc()),
        };
        let right = Expression::Literal {
            value: Literal::String("world".to_string()),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Add,
            right: Box::new(right),
            location: Some(test_loc()),
        };

        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_binary_no_strings() {
        // Test: a + b → false
        let left = Expression::Identifier {
            name: "a".to_string(),
            location: Some(test_loc()),
        };
        let right = Expression::Identifier {
            name: "b".to_string(),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Add,
            right: Box::new(right),
            location: Some(test_loc()),
        };

        assert!(!contains_string_literal(&expr));
    }

    #[test]
    fn test_nested_binary_with_string() {
        // Test: (a + b) + "hello" → true
        let a = Expression::Identifier {
            name: "a".to_string(),
            location: Some(test_loc()),
        };
        let b = Expression::Identifier {
            name: "b".to_string(),
            location: Some(test_loc()),
        };
        let ab = Expression::Binary {
            left: Box::new(a),
            op: BinaryOp::Add,
            right: Box::new(b),
            location: Some(test_loc()),
        };
        let str_lit = Expression::Literal {
            value: Literal::String("hello".to_string()),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(ab),
            op: BinaryOp::Add,
            right: Box::new(str_lit),
            location: Some(test_loc()),
        };

        assert!(contains_string_literal(&expr));
    }
}
