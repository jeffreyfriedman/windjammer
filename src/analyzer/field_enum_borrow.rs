//! Field-enum-match borrow heuristic (Copy aggregate readonly params).

use crate::parser::*;

fn is_in_receiver_chain(name: &str, expr: &Expression) -> bool {
    match expr {
        Expression::Identifier { name: id, .. } => id == name,
        Expression::FieldAccess { object, .. } => is_in_receiver_chain(name, object),
        Expression::MethodCall { object, .. } => is_in_receiver_chain(name, object),
        Expression::Index { object, .. } => is_in_receiver_chain(name, object),
        _ => false,
    }
}

fn expr_is_field_match_scrutinee_of_param(name: &str, expr: &Expression) -> bool {
    match expr {
        Expression::FieldAccess { object, .. } => is_in_receiver_chain(name, object),
        _ => false,
    }
}

fn stmt_scan_field_enum_match_scrutinee(
    name: &str,
    stmt: &Statement,
    saw_scrutinee: &mut bool,
    other_use: &mut bool,
) {
    if *other_use {
        return;
    }
    match stmt {
        Statement::Match { value, arms, .. } => {
            if expr_is_field_match_scrutinee_of_param(name, value) {
                *saw_scrutinee = true;
                for arm in arms {
                    expr_scan_field_enum_match_scrutinee(name, arm.body, saw_scrutinee, other_use);
                }
            } else {
                expr_scan_field_enum_match_scrutinee(name, value, saw_scrutinee, other_use);
                for arm in arms {
                    expr_scan_field_enum_match_scrutinee(name, arm.body, saw_scrutinee, other_use);
                }
            }
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            for s in then_block {
                stmt_scan_field_enum_match_scrutinee(name, s, saw_scrutinee, other_use);
            }
            if let Some(block) = else_block {
                for s in block {
                    stmt_scan_field_enum_match_scrutinee(name, s, saw_scrutinee, other_use);
                }
            }
        }
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Loop { body, .. } => {
            for s in body {
                stmt_scan_field_enum_match_scrutinee(name, s, saw_scrutinee, other_use);
            }
        }
        Statement::Return { value: Some(v), .. } => {
            expr_scan_field_enum_match_scrutinee(name, v, saw_scrutinee, other_use);
        }
        Statement::Expression { expr, .. } | Statement::Let { value: expr, .. } => {
            expr_scan_field_enum_match_scrutinee(name, expr, saw_scrutinee, other_use);
        }
        _ => {}
    }
}

fn expr_scan_field_enum_match_scrutinee(
    name: &str,
    expr: &Expression,
    saw_scrutinee: &mut bool,
    other_use: &mut bool,
) {
    if *other_use {
        return;
    }
    match expr {
        Expression::Identifier { name: id, .. } if id == name => {
            *other_use = true;
        }
        Expression::FieldAccess { object, .. } => {
            expr_scan_field_enum_match_scrutinee(name, object, saw_scrutinee, other_use);
        }
        Expression::MethodCall { object, arguments, .. } => {
            expr_scan_field_enum_match_scrutinee(name, object, saw_scrutinee, other_use);
            for (_, arg) in arguments {
                expr_scan_field_enum_match_scrutinee(name, arg, saw_scrutinee, other_use);
            }
        }
        Expression::Call { function, arguments, .. } => {
            expr_scan_field_enum_match_scrutinee(name, function, saw_scrutinee, other_use);
            for (_, arg) in arguments {
                expr_scan_field_enum_match_scrutinee(name, arg, saw_scrutinee, other_use);
            }
        }
        Expression::Binary { left, right, .. } => {
            expr_scan_field_enum_match_scrutinee(name, left, saw_scrutinee, other_use);
            expr_scan_field_enum_match_scrutinee(name, right, saw_scrutinee, other_use);
        }
        Expression::Unary { operand, .. } => {
            expr_scan_field_enum_match_scrutinee(name, operand, saw_scrutinee, other_use);
        }
        Expression::Index { object, index, .. } => {
            expr_scan_field_enum_match_scrutinee(name, object, saw_scrutinee, other_use);
            expr_scan_field_enum_match_scrutinee(name, index, saw_scrutinee, other_use);
        }
        Expression::Block { statements, .. } => {
            for s in statements {
                stmt_scan_field_enum_match_scrutinee(name, s, saw_scrutinee, other_use);
            }
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, v) in fields {
                expr_scan_field_enum_match_scrutinee(name, v, saw_scrutinee, other_use);
            }
        }
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            for el in elements {
                expr_scan_field_enum_match_scrutinee(name, el, saw_scrutinee, other_use);
            }
        }
        Expression::Cast { expr: inner, .. }
        | Expression::TryOp { expr: inner, .. }
        | Expression::Await { expr: inner, .. }
        | Expression::AsyncCall { expr: inner, .. }
        | Expression::SpawnCall { expr: inner, .. } => {
            expr_scan_field_enum_match_scrutinee(name, inner, saw_scrutinee, other_use);
        }
        Expression::Range { start, end, .. } => {
            expr_scan_field_enum_match_scrutinee(name, start, saw_scrutinee, other_use);
            expr_scan_field_enum_match_scrutinee(name, end, saw_scrutinee, other_use);
        }
        Expression::Closure { body, .. } => {
            expr_scan_field_enum_match_scrutinee(name, body, saw_scrutinee, other_use);
        }
        Expression::MapLiteral { pairs, .. } => {
            for (k, v) in pairs {
                expr_scan_field_enum_match_scrutinee(name, k, saw_scrutinee, other_use);
                expr_scan_field_enum_match_scrutinee(name, v, saw_scrutinee, other_use);
            }
        }
        Expression::MacroInvocation { args, .. } => {
            for arg in args {
                expr_scan_field_enum_match_scrutinee(name, arg, saw_scrutinee, other_use);
            }
        }
        Expression::ChannelSend { channel, value, .. } => {
            expr_scan_field_enum_match_scrutinee(name, channel, saw_scrutinee, other_use);
            expr_scan_field_enum_match_scrutinee(name, value, saw_scrutinee, other_use);
        }
        Expression::ChannelRecv { channel, .. } => {
            expr_scan_field_enum_match_scrutinee(name, channel, saw_scrutinee, other_use);
        }
        _ => {}
    }
}

/// True when every use of `name` is `match name.field { EnumVariant { .. } => ... }`.
pub(crate) fn param_only_used_as_field_enum_match_scrutinee<'ast>(
    name: &str,
    statements: &[&'ast Statement<'ast>],
) -> bool {
    let mut saw_scrutinee = false;
    let mut other_use = false;
    for stmt in statements {
        stmt_scan_field_enum_match_scrutinee(name, stmt, &mut saw_scrutinee, &mut other_use);
    }
    saw_scrutinee && !other_use
}

/// True when `name` is a bare argument to an enum variant constructor (`Shape::Named(name)`).
///
/// Used to keep concrete `String` formals (not `impl Into<String>`) when the value is
/// moved into a variant payload that is typed as owned `String`.
pub(crate) fn param_consumed_by_enum_variant_ctor<'ast>(
    name: &str,
    statements: &[&'ast Statement<'ast>],
) -> bool {
    statements
        .iter()
        .any(|stmt| stmt_has_enum_variant_consuming(name, stmt))
}

fn looks_like_enum_variant_constructor(fn_name: &str) -> bool {
    fn_name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
}

fn stmt_has_enum_variant_consuming(name: &str, stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::Let { value, .. } | Statement::Expression { expr: value, .. } => {
            expr_has_enum_variant_consuming(name, value)
        }
        Statement::Return {
            value: Some(expr), ..
        }
        | Statement::Assignment { value: expr, .. } => expr_has_enum_variant_consuming(name, expr),
        Statement::Match { value, arms, .. } => {
            expr_has_enum_variant_consuming(name, value)
                || arms
                    .iter()
                    .any(|arm| expr_has_enum_variant_consuming(name, arm.body))
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expr_has_enum_variant_consuming(name, condition)
                || then_block
                    .iter()
                    .any(|s| stmt_has_enum_variant_consuming(name, s))
                || else_block
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| stmt_has_enum_variant_consuming(name, s)))
        }
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Loop { body, .. } => {
            body.iter().any(|s| stmt_has_enum_variant_consuming(name, s))
        }
        _ => false,
    }
}

fn expr_has_enum_variant_consuming(name: &str, expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            let is_enum_variant = match &**function {
                Expression::Identifier { name: fn_name, .. } => {
                    looks_like_enum_variant_constructor(fn_name)
                }
                Expression::FieldAccess { field, .. } => {
                    field.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                }
                _ => false,
            };
            if is_enum_variant
                && arguments.iter().any(|(_label, arg)| {
                    matches!(arg, Expression::Identifier { name: id, .. } if id == name)
                })
            {
                return true;
            }
            arguments
                .iter()
                .any(|(_label, arg)| expr_has_enum_variant_consuming(name, arg))
                || expr_has_enum_variant_consuming(name, function)
        }
        Expression::MethodCall {
            method,
            arguments,
            object,
            ..
        } => {
            if method
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
                && arguments.iter().any(|(_label, arg)| {
                    matches!(arg, Expression::Identifier { name: id, .. } if id == name)
                })
            {
                return true;
            }
            arguments
                .iter()
                .any(|(_label, arg)| expr_has_enum_variant_consuming(name, arg))
                || expr_has_enum_variant_consuming(name, object)
        }
        Expression::Unary { operand, .. } => expr_has_enum_variant_consuming(name, operand),
        Expression::Block { statements, .. } => statements
            .iter()
            .any(|s| stmt_has_enum_variant_consuming(name, s)),
        Expression::Tuple { elements, .. } => elements
            .iter()
            .any(|el| expr_has_enum_variant_consuming(name, el)),
        Expression::StructLiteral { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_has_enum_variant_consuming(name, v)),
        _ => false,
    }
}
