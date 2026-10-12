//! Parameters captured by a closure handed to `spawn` must be owned.
//!
//! `thread::spawn(move || ..)` requires `'static` captures: a parameter used inside such a closure is
//! moved into the thread, so it cannot be a borrowed formal (`&Receiver<T>` is not `Send + 'static`).
//! This is a fact about the body (like "stored in a struct"); the ownership decision stays with the
//! parameter analysis that consumes it.

use crate::parser::{Expression, Statement};

fn is_spawn_callee(function: &Expression<'_>) -> bool {
    match function {
        Expression::Identifier { name, .. } => name == "spawn" || name.ends_with("::spawn"),
        Expression::FieldAccess { field, .. } => field == "spawn",
        _ => false,
    }
}

fn mentions(name: &str, expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Identifier { name: n, .. } => n == name,
        Expression::Binary { left, right, .. } => mentions(name, left) || mentions(name, right),
        Expression::Unary { operand, .. } => mentions(name, operand),
        Expression::Call { function, arguments, .. } => {
            mentions(name, function) || arguments.iter().any(|(_, a)| mentions(name, a))
        }
        Expression::MethodCall { object, arguments, .. } => {
            mentions(name, object) || arguments.iter().any(|(_, a)| mentions(name, a))
        }
        Expression::FieldAccess { object, .. } => mentions(name, object),
        Expression::Index { object, index, .. } => mentions(name, object) || mentions(name, index),
        Expression::Closure { body, .. } => mentions(name, body),
        Expression::Block { statements, .. } => statements.iter().any(|s| stmt_mentions(name, s)),
        Expression::TryOp { expr, .. } | Expression::Cast { expr, .. } | Expression::Await { expr, .. } => {
            mentions(name, expr)
        }
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            elements.iter().any(|e| mentions(name, e))
        }
        Expression::MacroInvocation { args, .. } => args.iter().any(|e| mentions(name, e)),
        Expression::StructLiteral { fields, .. } => fields.iter().any(|(_, e)| mentions(name, e)),
        _ => false,
    }
}

fn stmt_mentions(name: &str, stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::Let { value, .. } => mentions(name, value),
        Statement::Assignment { target, value, .. } => mentions(name, target) || mentions(name, value),
        Statement::Return { value: Some(v), .. } => mentions(name, v),
        Statement::Expression { expr, .. } => mentions(name, expr),
        Statement::If { condition, then_block, else_block, .. } => {
            mentions(name, condition)
                || then_block.iter().any(|s| stmt_mentions(name, s))
                || else_block.as_ref().is_some_and(|b| b.iter().any(|s| stmt_mentions(name, s)))
        }
        Statement::Match { value, arms, .. } => {
            mentions(name, value) || arms.iter().any(|a| mentions(name, a.body))
        }
        Statement::For { iterable, body, .. } => {
            mentions(name, iterable) || body.iter().any(|s| stmt_mentions(name, s))
        }
        Statement::While { condition, body, .. } => {
            mentions(name, condition) || body.iter().any(|s| stmt_mentions(name, s))
        }
        Statement::Loop { body, .. } | Statement::Thread { body, .. } | Statement::Async { body, .. } => {
            body.iter().any(|s| stmt_mentions(name, s))
        }
        _ => false,
    }
}

fn expr_has_spawn_capture(name: &str, expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Call { function, arguments, .. } => {
            if is_spawn_callee(function)
                && arguments
                    .iter()
                    .any(|(_, a)| matches!(a, Expression::Closure { body, .. } if mentions(name, body)))
            {
                return true;
            }
            arguments.iter().any(|(_, a)| expr_has_spawn_capture(name, a))
        }
        Expression::MethodCall { object, method, arguments, .. } => {
            // `thread::spawn(..)` is parsed as a method call on the module path.
            if method == "spawn"
                && arguments
                    .iter()
                    .any(|(_, a)| matches!(a, Expression::Closure { body, .. } if mentions(name, body)))
            {
                return true;
            }
            expr_has_spawn_capture(name, object) || arguments.iter().any(|(_, a)| expr_has_spawn_capture(name, a))
        }
        Expression::Block { statements, .. } => statements.iter().any(|s| stmt_has_spawn_capture(name, s)),
        Expression::Closure { body, .. } => expr_has_spawn_capture(name, body),
        Expression::TryOp { expr, .. } | Expression::Cast { expr, .. } | Expression::Await { expr, .. } => {
            expr_has_spawn_capture(name, expr)
        }
        Expression::SpawnCall { expr, .. } => mentions(name, expr),
        _ => false,
    }
}

fn stmt_has_spawn_capture(name: &str, stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::Let { value, .. } => expr_has_spawn_capture(name, value),
        Statement::Assignment { value, .. } => expr_has_spawn_capture(name, value),
        Statement::Return { value: Some(v), .. } => expr_has_spawn_capture(name, v),
        Statement::Expression { expr, .. } => expr_has_spawn_capture(name, expr),
        Statement::If { condition, then_block, else_block, .. } => {
            expr_has_spawn_capture(name, condition)
                || then_block.iter().any(|s| stmt_has_spawn_capture(name, s))
                || else_block.as_ref().is_some_and(|b| b.iter().any(|s| stmt_has_spawn_capture(name, s)))
        }
        Statement::Match { value, arms, .. } => {
            expr_has_spawn_capture(name, value) || arms.iter().any(|a| expr_has_spawn_capture(name, a.body))
        }
        Statement::For { iterable, body, .. } => {
            expr_has_spawn_capture(name, iterable) || body.iter().any(|s| stmt_has_spawn_capture(name, s))
        }
        Statement::While { condition, body, .. } => {
            expr_has_spawn_capture(name, condition) || body.iter().any(|s| stmt_has_spawn_capture(name, s))
        }
        Statement::Loop { body, .. } => body.iter().any(|s| stmt_has_spawn_capture(name, s)),
        Statement::Thread { body, .. } => body.iter().any(|s| stmt_mentions(name, s)),
        _ => false,
    }
}

/// True when `name` is used inside a closure passed to `spawn(..)` / `thread::spawn(..)`.
pub(crate) fn param_captured_by_spawned_closure(name: &str, body: &[&Statement<'_>]) -> bool {
    body.iter().any(|s| stmt_has_spawn_capture(name, s))
}

impl<'ast> super::Analyzer<'ast> {
    /// Base type name used to look up methods of a parameter's type in the signature registry
    /// (`Option`/`Result` methods such as `ok_or` are registered per base type).
    pub(crate) fn param_receiver_type_name<'t>(param_type: &'t crate::parser::Type) -> Option<&'t str> {
        use crate::parser::Type;
        match param_type {
            Type::Custom(name) | Type::Generic(name) => Some(name.split('<').next().unwrap_or(name.as_str())),
            Type::Option(_) => Some("Option"),
            Type::Result(..) => Some("Result"),
            _ => None,
        }
    }
}
