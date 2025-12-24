// Expression Analysis Helper Functions
//
// This module provides pure functions for analyzing expressions:
// - Reference detection (&x, &mut x)
// - Constant evaluation checking (compile-time evaluable expressions)

use crate::parser::{Expression, UnaryOp};

// =============================================================================
// Reference Detection
// =============================================================================

/// Check if an expression is a reference (&x or &mut x)
///
/// Returns true for both immutable and mutable references.
///
/// # Examples
/// ```
/// // &x → true
/// // &mut x → true
/// // !x → false
/// // x → false
/// ```
pub fn is_reference_expression(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Unary {
            op: UnaryOp::Ref | UnaryOp::MutRef,
            ..
        }
    )
}

// =============================================================================
// Constant Evaluation Detection
// =============================================================================

/// Check if an expression can be evaluated at compile time
///
/// Returns true for literals and expressions composed entirely of const values.
///
/// # Examples
/// ```
/// // 42 → true
/// // "hello" → true
/// // 1 + 2 → true
/// // -5 → true
/// // x → false
/// // x + 1 → false
/// ```
pub fn is_const_evaluable(expr: &Expression) -> bool {
    match expr {
        // Literals are always const
        Expression::Literal { .. } => true,

        // Binary operations on const values are const
        Expression::Binary { left, right, .. } => {
            is_const_evaluable(left) && is_const_evaluable(right)
        }

        // Unary operations on const values are const
        Expression::Unary { operand, .. } => is_const_evaluable(operand),

        // Struct literals with const fields might be const
        Expression::StructLiteral { fields, .. } => {
            fields.iter().all(|(_, expr)| is_const_evaluable(expr))
        }

        // Map literals with const entries might be const
        Expression::MapLiteral { pairs, .. } => pairs
            .iter()
            .all(|(key, val)| is_const_evaluable(key) && is_const_evaluable(val)),

        // Array literals with const elements are const
        Expression::Array { elements, .. } => elements.iter().all(is_const_evaluable),

        // Tuple literals with const elements are const
        Expression::Tuple { elements, .. } => elements.iter().all(is_const_evaluable),

        // Everything else (identifiers, calls, field access, etc.) is not const
        _ => false,
    }
}
