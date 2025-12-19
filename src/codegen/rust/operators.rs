// Operator mapping and precedence utilities
//
// This module provides pure functions for converting Windjammer operators
// to their Rust equivalents and determining operator precedence for
// expression generation.

use crate::parser::{BinaryOp, UnaryOp};

/// Maps a binary operator to its Rust syntax representation
///
/// # Examples
/// ```
/// use windjammer::parser::BinaryOp;
/// use windjammer::codegen::rust::operators::binary_op_to_rust;
///
/// assert_eq!(binary_op_to_rust(&BinaryOp::Add), "+");
/// assert_eq!(binary_op_to_rust(&BinaryOp::Eq), "==");
/// ```
pub fn binary_op_to_rust(op: &BinaryOp) -> &'static str {
    match op {
        // Arithmetic operators
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",

        // Comparison operators
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",

        // Logical operators
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",

        // Bitwise operators
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
    }
}

/// Maps a unary operator to its Rust syntax representation
///
/// # Examples
/// ```
/// use windjammer::parser::UnaryOp;
/// use windjammer::codegen::rust::operators::unary_op_to_rust;
///
/// assert_eq!(unary_op_to_rust(&UnaryOp::Not), "!");
/// assert_eq!(unary_op_to_rust(&UnaryOp::Ref), "&");
/// ```
pub fn unary_op_to_rust(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::Neg => "-",
        UnaryOp::Ref => "&",
        UnaryOp::MutRef => "&mut ",
        UnaryOp::Deref => "*",
    }
}

/// Returns the precedence level for a binary operator
///
/// Higher numbers indicate higher precedence (tighter binding).
/// This follows Rust's operator precedence rules:
/// - 10: Multiplicative (*, /, %)
/// - 9: Additive (+, -)
/// - 8: Bitshift (<<, >>)
/// - 7: Comparison (<, <=, >, >=)
/// - 6: Equality (==, !=)
/// - 5: Bitwise AND (&)
/// - 4: Bitwise XOR (^)
/// - 3: Bitwise OR (|)
/// - 2: Logical AND (&&)
/// - 1: Logical OR (||)
///
/// # Examples
/// ```
/// use windjammer::parser::BinaryOp;
/// use windjammer::codegen::rust::operators::op_precedence;
///
/// assert_eq!(op_precedence(&BinaryOp::Mul), 10); // Highest
/// assert_eq!(op_precedence(&BinaryOp::Add), 9);
/// assert_eq!(op_precedence(&BinaryOp::Or), 1);   // Lowest
/// ```
pub fn op_precedence(op: &BinaryOp) -> i32 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::BitOr => 3,
        BinaryOp::BitXor => 4,
        BinaryOp::BitAnd => 5,
        BinaryOp::Eq | BinaryOp::Ne => 6,
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 7,
        BinaryOp::Shl | BinaryOp::Shr => 8,
        BinaryOp::Add | BinaryOp::Sub => 9,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_binary_operators_covered() {
        // Ensure we handle all binary operators
        let ops = vec![
            BinaryOp::Add,
            BinaryOp::Sub,
            BinaryOp::Mul,
            BinaryOp::Div,
            BinaryOp::Mod,
            BinaryOp::Eq,
            BinaryOp::Ne,
            BinaryOp::Lt,
            BinaryOp::Le,
            BinaryOp::Gt,
            BinaryOp::Ge,
            BinaryOp::And,
            BinaryOp::Or,
            BinaryOp::BitAnd,
            BinaryOp::BitOr,
            BinaryOp::BitXor,
            BinaryOp::Shl,
            BinaryOp::Shr,
        ];

        for op in ops {
            // Should not panic
            let _ = binary_op_to_rust(&op);
            let _ = op_precedence(&op);
        }
    }

    #[test]
    fn test_all_unary_operators_covered() {
        // Ensure we handle all unary operators
        let ops = vec![
            UnaryOp::Not,
            UnaryOp::Neg,
            UnaryOp::Ref,
            UnaryOp::MutRef,
            UnaryOp::Deref,
        ];

        for op in ops {
            // Should not panic
            let _ = unary_op_to_rust(&op);
        }
    }
}

