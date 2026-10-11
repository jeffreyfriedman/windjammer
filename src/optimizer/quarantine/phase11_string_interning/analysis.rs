//! Frequency analysis for string literals eligible for interning.

use crate::parser::{Expression, Item, Literal, Program, Statement};
use std::collections::HashMap;

/// Analyze program and build string frequency map
pub(super) fn analyze_string_literals(program: &Program) -> HashMap<String, usize> {
    let mut frequency: HashMap<String, usize> = HashMap::new();

    for item in &program.items {
        collect_strings_from_item(item, &mut frequency);
    }

    frequency
}

/// Collect strings from an item
fn collect_strings_from_item(item: &Item, frequency: &mut HashMap<String, usize>) {
    match item {
        Item::Function { decl: func, .. } => {
            for stmt in &func.body {
                collect_strings_from_statement(stmt, frequency);
            }
        }
        Item::Impl {
            block: impl_block, ..
        } => {
            for func in &impl_block.functions {
                for stmt in &func.body {
                    collect_strings_from_statement(stmt, frequency);
                }
            }
        }
        Item::Static { value, .. } => {
            collect_strings_from_expression(value, frequency);
        }
        Item::Const { value, .. } => {
            collect_strings_from_expression(value, frequency);
        }
        _ => {}
    }
}

/// Collect strings from an expression recursively
fn collect_strings_from_expression(expr: &Expression, frequency: &mut HashMap<String, usize>) {
    match expr {
        // Only intern strings >= 10 characters
        Expression::Literal {
            value: Literal::String(s),
            ..
        } if s.len() >= 10 => {
            *frequency.entry(s.clone()).or_insert(0) += 1;
        }
        Expression::Binary { left, right, .. } => {
            collect_strings_from_expression(left, frequency);
            collect_strings_from_expression(right, frequency);
        }
        Expression::Unary { operand, .. } => {
            collect_strings_from_expression(operand, frequency);
        }
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            collect_strings_from_expression(function, frequency);
            for (_, arg) in arguments {
                collect_strings_from_expression(arg, frequency);
            }
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            collect_strings_from_expression(object, frequency);
            for (_, arg) in arguments {
                collect_strings_from_expression(arg, frequency);
            }
        }
        Expression::FieldAccess { object, .. } => {
            collect_strings_from_expression(object, frequency);
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                collect_strings_from_expression(value, frequency);
            }
        }
        Expression::Range { start, end, .. } => {
            collect_strings_from_expression(start, frequency);
            collect_strings_from_expression(end, frequency);
        }
        Expression::Closure { body, .. } => {
            collect_strings_from_expression(body, frequency);
        }
        Expression::Cast { expr, .. } => {
            collect_strings_from_expression(expr, frequency);
        }
        Expression::Index { object, index, .. } => {
            collect_strings_from_expression(object, frequency);
            collect_strings_from_expression(index, frequency);
        }
        Expression::Tuple { elements, .. } => {
            for elem in elements {
                collect_strings_from_expression(elem, frequency);
            }
        }
        Expression::MacroInvocation { args, .. } => {
            for arg in args {
                collect_strings_from_expression(arg, frequency);
            }
        }
        Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
            collect_strings_from_expression(expr, frequency);
        }
        Expression::ChannelSend { channel, value, .. } => {
            collect_strings_from_expression(channel, frequency);
            collect_strings_from_expression(value, frequency);
        }
        Expression::ChannelRecv { channel, .. } => {
            collect_strings_from_expression(channel, frequency);
        }
        Expression::Block { statements, .. } => {
            for stmt in statements {
                collect_strings_from_statement(stmt, frequency);
            }
        }
        _ => {}
    }
}

/// Collect strings from a statement
fn collect_strings_from_statement(stmt: &Statement, frequency: &mut HashMap<String, usize>) {
    match stmt {
        Statement::Let { value, .. }
        | Statement::Const { value, .. }
        | Statement::Static { value, .. } => {
            collect_strings_from_expression(value, frequency);
        }
        Statement::Expression { expr, .. } => {
            collect_strings_from_expression(expr, frequency);
        }
        Statement::Return {
            value: Some(expr), ..
        } => {
            collect_strings_from_expression(expr, frequency);
        }
        Statement::Assignment { target, value, .. } => {
            collect_strings_from_expression(target, frequency);
            collect_strings_from_expression(value, frequency);
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            collect_strings_from_expression(condition, frequency);
            for stmt in then_block {
                collect_strings_from_statement(stmt, frequency);
            }
            if let Some(else_stmts) = else_block {
                for stmt in else_stmts {
                    collect_strings_from_statement(stmt, frequency);
                }
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            collect_strings_from_expression(condition, frequency);
            for stmt in body {
                collect_strings_from_statement(stmt, frequency);
            }
        }
        Statement::For { iterable, body, .. } => {
            collect_strings_from_expression(iterable, frequency);
            for stmt in body {
                collect_strings_from_statement(stmt, frequency);
            }
        }
        Statement::Match { value, arms, .. } => {
            collect_strings_from_expression(value, frequency);
            for arm in arms {
                collect_strings_from_expression(arm.body, frequency);
            }
        }
        _ => {}
    }
}
