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
        Expression::Array { elements, .. } => elements.iter().all(|e| is_const_evaluable(e)),

        // Tuple literals with const elements are const
        Expression::Tuple { elements, .. } => elements.iter().all(|e| is_const_evaluable(e)),

        // Everything else (identifiers, calls, field access, etc.) is not const
        _ => false,
    }
}

// =============================================================================
// Identifier usage analysis (struct literals, move semantics)
// =============================================================================

/// Count how many times each identifier appears across struct literal field expressions.
pub fn count_identifier_usages_in_fields(
    fields: &[(String, &Expression<'_>)],
) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    for (_, expr) in fields {
        accumulate_identifier_usages(expr, &mut counts);
    }
    counts
}

fn accumulate_identifier_usages(
    expr: &Expression<'_>,
    counts: &mut std::collections::HashMap<String, usize>,
) {
    match expr {
        Expression::Identifier { name, .. } => {
            *counts.entry(name.clone()).or_default() += 1;
        }
        Expression::Binary { left, right, .. } => {
            accumulate_identifier_usages(left, counts);
            accumulate_identifier_usages(right, counts);
        }
        Expression::Unary { operand, .. } => accumulate_identifier_usages(operand, counts),
        Expression::FieldAccess { object, .. } => accumulate_identifier_usages(object, counts),
        Expression::Index { object, index, .. } => {
            accumulate_identifier_usages(object, counts);
            accumulate_identifier_usages(index, counts);
        }
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            accumulate_identifier_usages(function, counts);
            for (_, arg) in arguments {
                accumulate_identifier_usages(arg, counts);
            }
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            accumulate_identifier_usages(object, counts);
            for (_, arg) in arguments {
                accumulate_identifier_usages(arg, counts);
            }
        }
        Expression::MacroInvocation { args, .. } => {
            for arg in args {
                accumulate_identifier_usages(arg, counts);
            }
        }
        Expression::Block { statements, .. } => {
            for stmt in statements {
                if let crate::parser::Statement::Expression { expr, .. } = stmt {
                    accumulate_identifier_usages(expr, counts);
                }
            }
        }
        Expression::Array { elements, .. } | Expression::Tuple { elements, .. } => {
            for e in elements {
                accumulate_identifier_usages(e, counts);
            }
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, e) in fields {
                accumulate_identifier_usages(e, counts);
            }
        }
        Expression::MapLiteral { pairs, .. } => {
            for (k, v) in pairs {
                accumulate_identifier_usages(k, counts);
                accumulate_identifier_usages(v, counts);
            }
        }
        Expression::Range { start, end, .. } => {
            accumulate_identifier_usages(start, counts);
            accumulate_identifier_usages(end, counts);
        }
        Expression::Cast { expr, .. } => accumulate_identifier_usages(expr, counts),
        Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
            accumulate_identifier_usages(expr, counts);
        }
        Expression::ChannelSend { channel, value, .. } => {
            accumulate_identifier_usages(channel, counts);
            accumulate_identifier_usages(value, counts);
        }
        Expression::ChannelRecv { channel, .. } => accumulate_identifier_usages(channel, counts),
        Expression::Closure { body, .. } => accumulate_identifier_usages(body, counts),
        Expression::Literal { .. } => {}
    }
}
