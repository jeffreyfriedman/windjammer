//! Call-site scanning for which functions are referenced.

use crate::parser::{Expression, FunctionDecl, Item, Program, Statement};
use std::collections::HashSet;

/// Find all function calls in the program to determine which functions are used
pub(super) fn find_called_functions(program: &Program) -> HashSet<String> {
    let mut called = HashSet::new();

    // Main function is always considered "called" (entry point)
    called.insert("main".to_string());

    // Scan all items for function calls
    for item in &program.items {
        match item {
            Item::Function { decl: func, .. } => {
                find_calls_in_statements(&func.body, &mut called);
            }
            Item::Impl {
                block: impl_block, ..
            } => {
                for func in &impl_block.functions {
                    find_calls_in_statements(&func.body, &mut called);
                }
            }
            Item::Static { value, .. } | Item::Const { value, .. } => {
                find_calls_in_expression(value, &mut called);
            }
            _ => {}
        }
    }

    called
}

/// Find function calls in a list of statements
pub(super) fn find_calls_in_statements<'ast>(
    statements: &[&'ast Statement<'ast>],
    called: &mut HashSet<String>,
) {
    for stmt in statements {
        find_calls_in_statement(stmt, called);
    }
}

/// Find function calls in a statement
pub(super) fn find_calls_in_statement<'ast>(
    stmt: &'ast Statement<'ast>,
    called: &mut HashSet<String>,
) {
    match stmt {
        Statement::Expression { expr, .. } => {
            find_calls_in_expression(expr, called);
        }
        Statement::Return {
            value: Some(expr), ..
        } => {
            find_calls_in_expression(expr, called);
        }
        Statement::Return { value: None, .. } => {}
        Statement::Let { value, .. } => {
            find_calls_in_expression(value, called);
        }
        Statement::Assignment { value, .. } => {
            find_calls_in_expression(value, called);
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            find_calls_in_expression(condition, called);
            find_calls_in_statements(then_block, called);
            if let Some(else_stmts) = else_block {
                find_calls_in_statements(else_stmts, called);
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            find_calls_in_expression(condition, called);
            find_calls_in_statements(body, called);
        }
        Statement::For { iterable, body, .. } => {
            find_calls_in_expression(iterable, called);
            find_calls_in_statements(body, called);
        }
        Statement::Match { value, arms, .. } => {
            find_calls_in_expression(value, called);
            for arm in arms {
                find_calls_in_expression(arm.body, called);
                if let Some(guard) = &arm.guard {
                    find_calls_in_expression(guard, called);
                }
            }
        }
        Statement::Const { value, .. } | Statement::Static { value, .. } => {
            find_calls_in_expression(value, called);
        }
        _ => {}
    }
}

/// Find function calls in an expression
pub(super) fn find_calls_in_expression<'ast>(
    expr: &'ast Expression<'ast>,
    called: &mut HashSet<String>,
) {
    match expr {
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            // Track direct function calls
            if let Expression::Identifier { name, .. } = &**function {
                called.insert(name.clone());
            }
            find_calls_in_expression(function, called);
            for (_label, arg) in arguments {
                find_calls_in_expression(arg, called);
            }
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            find_calls_in_expression(object, called);
            for (_label, arg) in arguments {
                find_calls_in_expression(arg, called);
            }
        }
        Expression::Binary { left, right, .. } => {
            find_calls_in_expression(left, called);
            find_calls_in_expression(right, called);
        }
        Expression::Unary { operand, .. } => {
            find_calls_in_expression(operand, called);
        }
        Expression::Tuple { elements, .. } => {
            for elem in elements {
                find_calls_in_expression(elem, called);
            }
        }
        Expression::Index { object, index, .. } => {
            find_calls_in_expression(object, called);
            find_calls_in_expression(index, called);
        }
        Expression::FieldAccess { object, .. } => {
            find_calls_in_expression(object, called);
        }
        Expression::Cast { expr, .. } => {
            find_calls_in_expression(expr, called);
        }
        Expression::Block { statements, .. } => {
            find_calls_in_statements(statements, called);
        }
        Expression::Closure { body, .. } => {
            find_calls_in_expression(body, called);
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                find_calls_in_expression(value, called);
            }
        }
        Expression::Range { start, end, .. } => {
            find_calls_in_expression(start, called);
            find_calls_in_expression(end, called);
        }
        Expression::ChannelSend { channel, value, .. } => {
            find_calls_in_expression(channel, called);
            find_calls_in_expression(value, called);
        }
        Expression::ChannelRecv { channel, .. } => {
            find_calls_in_expression(channel, called);
        }
        Expression::Await { expr, .. } => {
            find_calls_in_expression(expr, called);
        }
        Expression::TryOp { expr, .. } => {
            find_calls_in_expression(expr, called);
        }
        Expression::MacroInvocation { args, .. } => {
            for arg in args {
                find_calls_in_expression(arg, called);
            }
        }
        _ => {}
    }
}

/// Check if a function is unused (private and never called)
pub(super) fn is_unused_function(func: &FunctionDecl, called_functions: &HashSet<String>) -> bool {
    // Functions with #[pub] decorator are always kept
    if func.decorators.iter().any(|d| d.name == "pub") {
        return false;
    }

    // Check if function is called
    !called_functions.contains(&func.name)
}
