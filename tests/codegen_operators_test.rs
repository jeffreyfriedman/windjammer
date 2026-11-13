#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

// TDD Tests for operator helper functions (Phase 4 - Expression Generation)
//
// This module contains pure functions for:
// - Binary operator to Rust string mapping
// - Unary operator to Rust string mapping
// - Operator precedence calculation

use windjammer::codegen::rust::operators::{binary_op_to_rust, op_precedence, unary_op_to_rust};
use windjammer::parser::{BinaryOp, UnaryOp};

// Tests will go here once we create the operators module
// For now, we test against the existing generator functions

#[cfg(test)]
mod binary_operator_tests {
    use super::*;

    #[test]
    fn test_arithmetic_operators() {
        // Arithmetic operators: +, -, *, /, %
        assert_eq!(binary_op_to_rust(&BinaryOp::Add), "+");
        assert_eq!(binary_op_to_rust(&BinaryOp::Sub), "-");
        assert_eq!(binary_op_to_rust(&BinaryOp::Mul), "*");
        assert_eq!(binary_op_to_rust(&BinaryOp::Div), "/");
        assert_eq!(binary_op_to_rust(&BinaryOp::Mod), "%");
    }

    #[test]
    fn test_comparison_operators() {
        // Comparison operators: ==, !=, <, <=, >, >=
        assert_eq!(binary_op_to_rust(&BinaryOp::Eq), "==");
        assert_eq!(binary_op_to_rust(&BinaryOp::Ne), "!=");
        assert_eq!(binary_op_to_rust(&BinaryOp::Lt), "<");
        assert_eq!(binary_op_to_rust(&BinaryOp::Le), "<=");
        assert_eq!(binary_op_to_rust(&BinaryOp::Gt), ">");
        assert_eq!(binary_op_to_rust(&BinaryOp::Ge), ">=");
    }

    #[test]
    fn test_logical_operators() {
        // Logical operators: &&, ||
        assert_eq!(binary_op_to_rust(&BinaryOp::And), "&&");
        assert_eq!(binary_op_to_rust(&BinaryOp::Or), "||");
    }

    #[test]
    fn test_bitwise_operators() {
        // Bitwise operators: &, |, ^, <<, >>
        assert_eq!(binary_op_to_rust(&BinaryOp::BitAnd), "&");
        assert_eq!(binary_op_to_rust(&BinaryOp::BitOr), "|");
        assert_eq!(binary_op_to_rust(&BinaryOp::BitXor), "^");
        assert_eq!(binary_op_to_rust(&BinaryOp::Shl), "<<");
        assert_eq!(binary_op_to_rust(&BinaryOp::Shr), ">>");
    }
}

#[cfg(test)]
mod unary_operator_tests {
    use super::*;

    #[test]
    fn test_logical_not() {
        // Logical NOT: !
        assert_eq!(unary_op_to_rust(&UnaryOp::Not), "!");
    }

    #[test]
    fn test_negation() {
        // Negation: -
        assert_eq!(unary_op_to_rust(&UnaryOp::Neg), "-");
    }

    #[test]
    fn test_reference_operators() {
        // Reference: &, &mut
        assert_eq!(unary_op_to_rust(&UnaryOp::Ref), "&");
        assert_eq!(unary_op_to_rust(&UnaryOp::MutRef), "&mut ");
    }

    #[test]
    fn test_dereference() {
        // Dereference: *
        assert_eq!(unary_op_to_rust(&UnaryOp::Deref), "*");
    }
}

#[cfg(test)]
mod operator_precedence_tests {
    use super::*;

    /// The Rust reference's table, tightest first (operators in one row share a level). The output must follow it,
    /// whatever the Windjammer parser's own table is (SPEC-EXPR-001), so the tree the parser built is the tree rustc reads.
    fn rust_levels() -> Vec<Vec<BinaryOp>> {
        vec![
            vec![BinaryOp::Mul, BinaryOp::Div, BinaryOp::Mod],
            vec![BinaryOp::Add, BinaryOp::Sub],
            vec![BinaryOp::Shl, BinaryOp::Shr],
            vec![BinaryOp::BitAnd],
            vec![BinaryOp::BitXor],
            vec![BinaryOp::BitOr],
            vec![BinaryOp::Eq, BinaryOp::Ne, BinaryOp::Lt, BinaryOp::Le, BinaryOp::Gt, BinaryOp::Ge],
            vec![BinaryOp::And],
            vec![BinaryOp::Or],
        ]
    }

    #[test]
    fn test_precedence_matches_the_rust_reference_table() {
        let levels = rust_levels();
        for (i, row) in levels.iter().enumerate() {
            for op in row {
                assert_eq!(op_precedence(op), op_precedence(&row[0]), "{op:?} shares its level");
            }
            if let Some(next) = levels.get(i + 1) {
                assert!(op_precedence(&row[0]) > op_precedence(&next[0]), "{:?} binds tighter than {:?}", row[0], next[0]);
            }
        }
    }

    #[test]
    fn test_precedence_ordering() {
        // multiplication > addition > shifts > bitwise > comparison > logical
        assert!(op_precedence(&BinaryOp::Mul) > op_precedence(&BinaryOp::Add));
        assert!(op_precedence(&BinaryOp::Add) > op_precedence(&BinaryOp::Shl));
        assert!(op_precedence(&BinaryOp::Shl) > op_precedence(&BinaryOp::BitAnd));
        assert!(op_precedence(&BinaryOp::BitAnd) > op_precedence(&BinaryOp::Eq));
        assert!(op_precedence(&BinaryOp::Lt) > op_precedence(&BinaryOp::And));
        assert!(op_precedence(&BinaryOp::And) > op_precedence(&BinaryOp::Or));
    }
}
