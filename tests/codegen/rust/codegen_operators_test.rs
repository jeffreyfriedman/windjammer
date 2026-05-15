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

    #[test]
    fn test_precedence_level_1_logical_or() {
        // Lowest precedence: ||
        assert_eq!(op_precedence(&BinaryOp::Or), 1);
    }

    #[test]
    fn test_precedence_level_2_logical_and() {
        // Level 2: &&
        assert_eq!(op_precedence(&BinaryOp::And), 2);
    }

    #[test]
    fn test_precedence_level_3_bitwise_or() {
        // Level 3: |
        assert_eq!(op_precedence(&BinaryOp::BitOr), 3);
    }

    #[test]
    fn test_precedence_level_4_bitwise_xor() {
        // Level 4: ^
        assert_eq!(op_precedence(&BinaryOp::BitXor), 4);
    }

    #[test]
    fn test_precedence_level_5_bitwise_and() {
        // Level 5: &
        assert_eq!(op_precedence(&BinaryOp::BitAnd), 5);
    }

    #[test]
    fn test_precedence_level_6_equality() {
        // Level 6: ==, !=
        assert_eq!(op_precedence(&BinaryOp::Eq), 6);
        assert_eq!(op_precedence(&BinaryOp::Ne), 6);
    }

    #[test]
    fn test_precedence_level_7_comparison() {
        // Level 7: <, <=, >, >=
        assert_eq!(op_precedence(&BinaryOp::Lt), 7);
        assert_eq!(op_precedence(&BinaryOp::Le), 7);
        assert_eq!(op_precedence(&BinaryOp::Gt), 7);
        assert_eq!(op_precedence(&BinaryOp::Ge), 7);
    }

    #[test]
    fn test_precedence_level_8_bitshift() {
        // Level 8: <<, >>
        assert_eq!(op_precedence(&BinaryOp::Shl), 8);
        assert_eq!(op_precedence(&BinaryOp::Shr), 8);
    }

    #[test]
    fn test_precedence_level_9_additive() {
        // Level 9: +, -
        assert_eq!(op_precedence(&BinaryOp::Add), 9);
        assert_eq!(op_precedence(&BinaryOp::Sub), 9);
    }

    #[test]
    fn test_precedence_level_10_multiplicative() {
        // Highest precedence: *, /, %
        assert_eq!(op_precedence(&BinaryOp::Mul), 10);
        assert_eq!(op_precedence(&BinaryOp::Div), 10);
        assert_eq!(op_precedence(&BinaryOp::Mod), 10);
    }

    #[test]
    fn test_precedence_ordering() {
        // Verify relative ordering: multiplication > addition > comparison > logical
        assert!(op_precedence(&BinaryOp::Mul) > op_precedence(&BinaryOp::Add));
        assert!(op_precedence(&BinaryOp::Add) > op_precedence(&BinaryOp::Lt));
        assert!(op_precedence(&BinaryOp::Lt) > op_precedence(&BinaryOp::And));
        assert!(op_precedence(&BinaryOp::And) > op_precedence(&BinaryOp::Or));
    }
}
