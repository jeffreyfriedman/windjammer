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

/// Returns the precedence level for a binary operator **as Rust parses it**.
///
/// Used by Rust codegen to decide when to emit parentheses. Windjammer's parser differs from Rust (shifts bind
/// tighter than additive; `& ^ |` bind looser than comparisons, SPEC-EXPR-001); emission must follow Rust's table so
/// that the tree the Windjammer parser built is the tree rustc reads back: `Add(a, Shl(b, c))` is `a + (b << c)` and
/// `BitAnd(a, Eq(b, c))` is `a & (b == c)`.
///
/// Higher numbers bind tighter (Rust reference, operator precedence):
/// - 9: `* / %`
/// - 8: `+ -`
/// - 7: `<< >>`
/// - 6: `&`
/// - 5: `^`
/// - 4: `|`
/// - 3: `== != < <= > >=` (non-associative)
/// - 2: `&&`
/// - 1: `||`
///
/// # Examples
/// ```
/// use windjammer::parser::BinaryOp;
/// use windjammer::codegen::rust::operators::op_precedence;
///
/// assert!(op_precedence(&BinaryOp::Mul) > op_precedence(&BinaryOp::Add));
/// assert!(op_precedence(&BinaryOp::Add) > op_precedence(&BinaryOp::Shl));
/// assert!(op_precedence(&BinaryOp::BitAnd) > op_precedence(&BinaryOp::Eq));
/// assert!(op_precedence(&BinaryOp::And) > op_precedence(&BinaryOp::Or));
/// ```
pub fn op_precedence(op: &BinaryOp) -> i32 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 3,
        BinaryOp::BitOr => 4,
        BinaryOp::BitXor => 5,
        BinaryOp::BitAnd => 6,
        BinaryOp::Shl | BinaryOp::Shr => 7,
        BinaryOp::Add | BinaryOp::Sub => 8,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 9,
    }
}

/// A bitwise operator under a comparison: Rust reads it the same way (`a & b == c` is `(a & b) == c`) but it is
/// easy to misread, so the parentheses stay.
pub fn bitwise_under_comparison(parent: &BinaryOp, child: &BinaryOp) -> bool {
    matches!(parent, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge)
        && matches!(child, BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor)
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
