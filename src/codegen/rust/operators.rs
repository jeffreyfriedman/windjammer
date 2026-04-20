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

/// When the parent and right-hand child are binary operators with the **same** Rust precedence,
/// `op_precedence(child) < op_precedence(parent)` is false, so the naive codegen omits parens.
/// Rust parses `*`, `/`, `%` and `+`, `-` as **left-associative** at each level, so the RHS
/// often must stay wrapped: e.g. `x / (2.0 * y)` must not become `x / 2.0 * y`.
///
/// Returns true when the RHS subexpression must be wrapped in parentheses.
pub fn binary_rhs_needs_parens_for_rust_left_assoc(
    parent: &BinaryOp,
    right_child: &BinaryOp,
) -> bool {
    if op_precedence(parent) != op_precedence(right_child) {
        return false;
    }
    match (parent, right_child) {
        // `a * b * c` ≡ `(a * b) * c` ≡ `a * (b * c)` for plain multiplication — no parens needed.
        // Every other same-precedence mix on the RHS changes meaning without parens.
        (
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod,
        ) => !matches!((parent, right_child), (BinaryOp::Mul, BinaryOp::Mul)),

        // Only `a + (b + c)` is redundant; `-` and mixed `+`/`-` need parens on the RHS.
        (BinaryOp::Add | BinaryOp::Sub, BinaryOp::Add | BinaryOp::Sub) => {
            !matches!((parent, right_child), (BinaryOp::Add, BinaryOp::Add))
        }

        // Shifts are left-associative; `a << (b << c)` must not become `a << b << c`.
        (BinaryOp::Shl | BinaryOp::Shr, BinaryOp::Shl | BinaryOp::Shr) => true,

        _ => false,
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

    #[test]
    fn test_left_assoc_rhs_parens_div_mul() {
        assert!(binary_rhs_needs_parens_for_rust_left_assoc(
            &BinaryOp::Div,
            &BinaryOp::Mul
        ));
        assert!(!binary_rhs_needs_parens_for_rust_left_assoc(
            &BinaryOp::Mul,
            &BinaryOp::Mul
        ));
    }

    #[test]
    fn test_left_assoc_rhs_parens_add_sub() {
        assert!(binary_rhs_needs_parens_for_rust_left_assoc(
            &BinaryOp::Add,
            &BinaryOp::Sub
        ));
        assert!(!binary_rhs_needs_parens_for_rust_left_assoc(
            &BinaryOp::Add,
            &BinaryOp::Add
        ));
    }
}
