//! Match Arm String Analysis
//!
//! Provides analysis functions for determining if match arms return
//! string literals that need conversion (`.to_string()`).
//!
//! This is used during code generation to decide whether to wrap
//! match expressions in string conversion.

use crate::parser::{Expression, Literal, Statement};

/// Check if a match arm returns a converted string literal
///
/// Returns `true` if the expression is a block that ends with:
/// 1. An if-else where both branches return string literals, OR
/// 2. A direct string literal expression
///
/// This is used to determine if match arms need `.to_string()` conversion.
///
/// # Arguments
/// * `expr` - The expression to analyze (typically a match arm body)
///
/// # Returns
/// * `true` - Expression returns a string literal needing conversion
/// * `false` - Expression does not return a convertible string literal
///
/// # Examples
/// ```ignore
/// // Returns true:
/// { if cond { "yes" } else { "no" } }
/// { "hello" }
///
/// // Returns false:
/// { if cond { "yes" } else { 42 } }
/// { 42 }
/// "hello" // Not in a block
/// ```
pub fn arm_returns_converted_string(expr: &Expression) -> bool {
    match expr {
        // Block with if-else that returns strings
        Expression::Block { statements, .. } => {
            if let Some(last) = statements.last() {
                match last {
                    Statement::If {
                        then_block,
                        else_block,
                        ..
                    } => {
                        // Check if both branches have string literals
                        let then_has_string = then_block.last().is_some_and(|s| {
                            matches!(s, Statement::Expression { expr: e, .. }
                                if matches!(e, Expression::Literal { value: Literal::String(_), .. }))
                        });
                        let else_has_string = else_block.as_ref().is_some_and(|block| {
                            block.last().is_some_and(|s| {
                                matches!(s, Statement::Expression { expr: e, .. }
                                    if matches!(e, Expression::Literal { value: Literal::String(_), .. }))
                            })
                        });
                        then_has_string && else_has_string
                    }
                    Statement::Expression { expr: e, .. } => {
                        // Check if it's a string literal (will be converted)
                        // OR recursively check nested expressions
                        matches!(
                            e,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) || arm_returns_converted_string(e)
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        // Direct if-else expression (not in a block)
        _ => false,
    }
}
