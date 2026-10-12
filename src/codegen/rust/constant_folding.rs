//! Constant Folding Optimization
//!
//! Performs compile-time evaluation of constant expressions to optimize
//! generated code. This is a classic compiler optimization that reduces
//! runtime computation by folding constant operations at compile time.
//!
//! # Examples
//! - `2 + 3` → `5`
//! - `10 * 2 + 5` → `25`
//! - `true && false` → `false`
//! - `-42` → `-42`
//!
//! # Safety
//! - Does not fold division/modulo by zero
//! - Returns `None` for non-foldable expressions
//! - Preserves type safety (no mixed-type folding)

use crate::parser::{BinaryOp, Expression, Literal, UnaryOp};

/// Try to fold a constant expression at compile time
///
/// Recursively evaluates constant expressions and returns a folded literal
/// if the expression can be evaluated at compile time. Returns `None` if
/// the expression cannot be folded (non-constant, division by zero, etc.).
///
/// # Arguments
/// * `expr` - The expression to attempt to fold
///
/// # Returns
/// * `Some(Expression::Literal)` - The folded constant value
/// * `None` - Expression cannot be folded
///
/// # Examples
/// ```ignore
/// let expr = Binary { left: Int(2), op: Add, right: Int(3) };
/// let folded = try_fold_constant(&expr); // Some(Int(5))
/// ```
pub fn try_fold_constant<'ast>(expr: &Expression<'ast>) -> Option<Expression<'ast>> {
    match expr {
        Expression::Binary {
            left,
            op,
            right,
            location,
            ..
        } => {
            // Try to fold both sides first (recursive)
            let left_folded = try_fold_constant(left).unwrap_or_else(|| (**left).clone());
            let right_folded = try_fold_constant(right).unwrap_or_else(|| (**right).clone());

            // If both sides are literals, try to evaluate
            if let (Expression::Literal { value: l, .. }, Expression::Literal { value: r, .. }) =
                (&left_folded, &right_folded)
            {
                use BinaryOp::*;
                use Literal::*;

                let result = match (l, op, r) {
                    // Integer arithmetic
                    (Int(a), Add, Int(b)) => Some(Literal::Int(a + b)),
                    (Int(a), Sub, Int(b)) => Some(Literal::Int(a - b)),
                    (Int(a), Mul, Int(b)) => Some(Literal::Int(a * b)),
                    (Int(a), Div, Int(b)) if *b != 0 => Some(Literal::Int(a / b)),
                    (Int(a), Mod, Int(b)) if *b != 0 => Some(Literal::Int(a % b)),

                    // Float arithmetic
                    // Folded only when the result is finite and identical in f32 and f64 arithmetic
                    // (the literal's width is decided later); otherwise rustc folds it at the right width.
                    (Float(a), Add, Float(b)) => fold_float(*a, *b, |x, y| x + y, |x, y| x + y),
                    (Float(a), Sub, Float(b)) => fold_float(*a, *b, |x, y| x - y, |x, y| x - y),
                    (Float(a), Mul, Float(b)) => fold_float(*a, *b, |x, y| x * y, |x, y| x * y),
                    (Float(a), Div, Float(b)) if *b != 0.0 => fold_float(*a, *b, |x, y| x / y, |x, y| x / y),

                    // Integer comparisons
                    (Int(a), Eq, Int(b)) => Some(Literal::Bool(a == b)),
                    (Int(a), Ne, Int(b)) => Some(Literal::Bool(a != b)),
                    (Int(a), Lt, Int(b)) => Some(Literal::Bool(a < b)),
                    (Int(a), Le, Int(b)) => Some(Literal::Bool(a <= b)),
                    (Int(a), Gt, Int(b)) => Some(Literal::Bool(a > b)),
                    (Int(a), Ge, Int(b)) => Some(Literal::Bool(a >= b)),

                    // Boolean operations
                    (Bool(a), And, Bool(b)) => Some(Literal::Bool(*a && *b)),
                    (Bool(a), Or, Bool(b)) => Some(Literal::Bool(*a || *b)),

                    _ => None,
                };

                // TDD FIX: Preserve location from original Binary expression
                // This ensures folded literals maintain the same ExprId during type inference,
                // allowing constraints to propagate correctly through regeneration passes
                return result.map(|value| Expression::Literal {
                    value,
                    location: location.clone(),
                });
            }
            None
        }
        Expression::Unary {
            op,
            operand,
            location,
            ..
        } => {
            // Try to fold operand first (recursive)
            let operand_folded = try_fold_constant(operand).unwrap_or_else(|| (**operand).clone());

            if let Expression::Literal { value: lit, .. } = &operand_folded {
                use Literal::*;
                use UnaryOp::*;

                let result = match (op, lit) {
                    (Neg, Int(n)) => Some(Literal::Int(-n)),
                    (Neg, Float(f)) if *f != 0.0 => Some(Literal::Float(-f)),
                    (Not, Bool(b)) => Some(Literal::Bool(!b)),
                    _ => None,
                };

                // TDD FIX: Preserve location from original Unary expression
                return result.map(|value| Expression::Literal {
                    value,
                    location: location.clone(),
                });
            }
            None
        }
        // Already a literal - can't fold further
        Expression::Literal { .. } => None,
        // Can't fold non-constant expressions
        _ => None,
    }
}

/// Width-independent float fold: `Some` only if f64 and f32 arithmetic agree and the result is finite.
fn fold_float(a: f64, b: f64, f64op: fn(f64, f64) -> f64, f32op: fn(f32, f32) -> f32) -> Option<Literal> {
    let wide = f64op(a, b);
    let narrow = f32op(a as f32, b as f32);
    (wide.is_finite() && wide != 0.0 && f64::from(narrow) == wide && f64::from(a as f32) == a && f64::from(b as f32) == b)
        .then_some(Literal::Float(wide))
}

#[cfg(test)]
mod float_fold_tests {
    use super::fold_float;
    #[test]
    fn folds_only_width_independent_results() {
        assert!(fold_float(3.5, 2.5, |x, y| x + y, |x, y| x + y).is_some());
        assert!(fold_float(0.1, 0.2, |x, y| x + y, |x, y| x + y).is_none());
        assert!(fold_float(1e300, 1e300, |x, y| x * y, |x, y| x * y).is_none());
        assert!(fold_float(1.0, -1.0, |x, y| x + y, |x, y| x + y).is_none());
    }
}
