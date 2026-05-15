//! Loop detection and analysis helpers (variable use / dependency checks).

use crate::parser::{Expression, Pattern, Statement};

/// Check if an expression uses a specific variable
pub(in crate::optimizer) fn expression_uses_variable<'ast>(
    expr: &'ast Expression<'ast>,
    var_name: &str,
) -> bool {
    match expr {
        Expression::Identifier { name, .. } => name == var_name,
        Expression::Binary { left, right, .. } => {
            expression_uses_variable(left, var_name) || expression_uses_variable(right, var_name)
        }
        Expression::Unary { operand, .. } => expression_uses_variable(operand, var_name),
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            expression_uses_variable(function, var_name)
                || arguments
                    .iter()
                    .any(|(_, arg)| expression_uses_variable(arg, var_name))
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            expression_uses_variable(object, var_name)
                || arguments
                    .iter()
                    .any(|(_, arg)| expression_uses_variable(arg, var_name))
        }
        Expression::Index { object, index, .. } => {
            expression_uses_variable(object, var_name) || expression_uses_variable(index, var_name)
        }
        Expression::FieldAccess { object, .. } => expression_uses_variable(object, var_name),
        Expression::Cast { expr, .. } => expression_uses_variable(expr, var_name),
        Expression::Tuple { elements, .. } => elements
            .iter()
            .any(|e| expression_uses_variable(e, var_name)),
        Expression::StructLiteral { fields, .. } => fields
            .iter()
            .any(|(_, v)| expression_uses_variable(v, var_name)),
        Expression::Range { start, end, .. } => {
            expression_uses_variable(start, var_name) || expression_uses_variable(end, var_name)
        }
        Expression::Closure { body, .. } => expression_uses_variable(body, var_name),
        Expression::Block { statements, .. } => statements
            .iter()
            .any(|s| statement_uses_variable(s, var_name)),
        Expression::ChannelSend { channel, value, .. } => {
            expression_uses_variable(channel, var_name) || expression_uses_variable(value, var_name)
        }
        Expression::ChannelRecv { channel, .. } => expression_uses_variable(channel, var_name),
        Expression::Await { expr, .. } | Expression::TryOp { expr, .. } => {
            expression_uses_variable(expr, var_name)
        }
        Expression::MacroInvocation { args, .. } => {
            args.iter().any(|a| expression_uses_variable(a, var_name))
        }
        _ => false,
    }
}

/// Check if a statement uses a specific variable
pub(in crate::optimizer) fn statement_uses_variable<'ast>(
    stmt: &'ast Statement<'ast>,
    var_name: &str,
) -> bool {
    match stmt {
        Statement::Expression { expr, .. }
        | Statement::Return {
            value: Some(expr), ..
        } => expression_uses_variable(expr, var_name),
        Statement::Let { value, .. } => expression_uses_variable(value, var_name),
        Statement::Assignment { target, value, .. } => {
            expression_uses_variable(target, var_name) || expression_uses_variable(value, var_name)
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_uses_variable(condition, var_name)
                || then_block
                    .iter()
                    .any(|s| statement_uses_variable(s, var_name))
                || else_block
                    .as_ref()
                    .is_some_and(|stmts| stmts.iter().any(|s| statement_uses_variable(s, var_name)))
        }
        Statement::While {
            condition, body, ..
        } => {
            expression_uses_variable(condition, var_name)
                || body.iter().any(|s| statement_uses_variable(s, var_name))
        }
        Statement::For {
            pattern,
            iterable,
            body,
            ..
        } => {
            // If this is a nested loop with the same variable, it shadows the outer one
            // For tuple patterns, we're conservative and assume it might shadow
            let shadows = match pattern {
                Pattern::Identifier(var) => var == var_name,
                Pattern::Tuple(_) => true, // Conservative: assume tuple might contain the variable
                _ => false,
            };
            if shadows {
                return false;
            }
            expression_uses_variable(iterable, var_name)
                || body.iter().any(|s| statement_uses_variable(s, var_name))
        }
        Statement::Match { value, arms, .. } => {
            expression_uses_variable(value, var_name)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .is_some_and(|g| expression_uses_variable(g, var_name))
                        || expression_uses_variable(arm.body, var_name)
                })
        }
        _ => false,
    }
}
