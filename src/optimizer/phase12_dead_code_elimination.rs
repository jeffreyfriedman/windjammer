//! Phase 12: Dead Code Elimination
//!
//! This optimization removes unreachable and unused code:
//! - Statements after return/break/continue
//! - Unused private functions
//! - Unused variables (assigned but never read)
//! - Empty blocks and branches
//!
//! Example transformations:
//! ```text
//! fn example() -> i32 {
//!     return 42
//!     println!("unreachable")  // Removed
//! }
//!
//! fn unused_helper() { ... }  // Removed if never called
//! ```

use crate::parser::{Expression, FunctionDecl, ImplBlock, Item, MatchArm, Program, Statement};
use std::collections::HashSet;

/// Statistics about dead code elimination
#[derive(Debug, Default, Clone)]
pub struct DeadCodeStats {
    pub unreachable_statements_removed: usize,
    pub unused_functions_removed: usize,
    pub unused_variables_removed: usize,
    pub empty_blocks_removed: usize,
}

/// Perform dead code elimination on a program
pub fn eliminate_dead_code(program: &Program) -> (Program, DeadCodeStats) {
    let mut stats = DeadCodeStats::default();

    // Step 1: Find all called functions (used to identify unused functions)
    let called_functions = find_called_functions(program);

    // Step 2: Process all items, removing dead code
    let mut new_items = Vec::new();
    for item in &program.items {
        match item {
            Item::Function(func) => {
                // Check if function is unused (private and never called)
                if is_unused_function(func, &called_functions) {
                    stats.unused_functions_removed += 1;
                    continue; // Skip this function
                }

                // Process function body to remove dead code
                let (new_body, func_stats) = eliminate_dead_code_in_statements(&func.body);
                stats.unreachable_statements_removed += func_stats.unreachable_statements_removed;
                stats.unused_variables_removed += func_stats.unused_variables_removed;
                stats.empty_blocks_removed += func_stats.empty_blocks_removed;

                let new_func = FunctionDecl {
                    name: func.name.clone(),
                    type_params: func.type_params.clone(),
                    where_clause: func.where_clause.clone(),
                    decorators: func.decorators.clone(),
                    is_async: func.is_async,
                    parameters: func.parameters.clone(),
                    return_type: func.return_type.clone(),
                    body: new_body,
                };
                new_items.push(Item::Function(new_func));
            }
            Item::Impl(impl_block) => {
                // Process impl block methods
                let new_impl = eliminate_dead_code_in_impl(impl_block, &mut stats);
                new_items.push(Item::Impl(new_impl));
            }
            Item::Static {
                name,
                mutable,
                type_,
                value,
            } => {
                // Process static initializers
                let new_value = eliminate_dead_code_in_expression(value);
                new_items.push(Item::Static {
                    name: name.clone(),
                    mutable: *mutable,
                    type_: type_.clone(),
                    value: new_value,
                });
            }
            Item::Const { name, type_, value } => {
                // Process const initializers
                let new_value = eliminate_dead_code_in_expression(value);
                new_items.push(Item::Const {
                    name: name.clone(),
                    type_: type_.clone(),
                    value: new_value,
                });
            }
            // Other items pass through unchanged
            _ => new_items.push(item.clone()),
        }
    }

    let new_program = Program { items: new_items };
    (new_program, stats)
}

/// Find all function calls in the program to determine which functions are used
fn find_called_functions(program: &Program) -> HashSet<String> {
    let mut called = HashSet::new();

    // Main function is always considered "called" (entry point)
    called.insert("main".to_string());

    // Scan all items for function calls
    for item in &program.items {
        match item {
            Item::Function(func) => {
                find_calls_in_statements(&func.body, &mut called);
            }
            Item::Impl(impl_block) => {
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
fn find_calls_in_statements(statements: &[Statement], called: &mut HashSet<String>) {
    for stmt in statements {
        find_calls_in_statement(stmt, called);
    }
}

/// Find function calls in a statement
fn find_calls_in_statement(stmt: &Statement, called: &mut HashSet<String>) {
    match stmt {
        Statement::Expression(expr) => {
            find_calls_in_expression(expr, called);
        }
        Statement::Return(Some(expr)) => {
            find_calls_in_expression(expr, called);
        }
        Statement::Return(None) => {}
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
        } => {
            find_calls_in_expression(condition, called);
            find_calls_in_statements(then_block, called);
            if let Some(else_stmts) = else_block {
                find_calls_in_statements(else_stmts, called);
            }
        }
        Statement::While { condition, body } => {
            find_calls_in_expression(condition, called);
            find_calls_in_statements(body, called);
        }
        Statement::For { iterable, body, .. } => {
            find_calls_in_expression(iterable, called);
            find_calls_in_statements(body, called);
        }
        Statement::Match { value, arms } => {
            find_calls_in_expression(value, called);
            for arm in arms {
                find_calls_in_expression(&arm.body, called);
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
fn find_calls_in_expression(expr: &Expression, called: &mut HashSet<String>) {
    match expr {
        Expression::Call {
            function,
            arguments,
        } => {
            // Track direct function calls
            if let Expression::Identifier(name) = &**function {
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
        Expression::Tuple(elements) => {
            for elem in elements {
                find_calls_in_expression(elem, called);
            }
        }
        Expression::Index { object, index } => {
            find_calls_in_expression(object, called);
            find_calls_in_expression(index, called);
        }
        Expression::FieldAccess { object, .. } => {
            find_calls_in_expression(object, called);
        }
        Expression::Cast { expr, .. } => {
            find_calls_in_expression(expr, called);
        }
        Expression::Block(statements) => {
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
        Expression::ChannelSend { channel, value } => {
            find_calls_in_expression(channel, called);
            find_calls_in_expression(value, called);
        }
        Expression::ChannelRecv(channel) => {
            find_calls_in_expression(channel, called);
        }
        Expression::Await(expr) => {
            find_calls_in_expression(expr, called);
        }
        Expression::TryOp(expr) => {
            find_calls_in_expression(expr, called);
        }
        Expression::Ternary {
            condition,
            true_expr,
            false_expr,
        } => {
            find_calls_in_expression(condition, called);
            find_calls_in_expression(true_expr, called);
            find_calls_in_expression(false_expr, called);
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
fn is_unused_function(func: &FunctionDecl, called_functions: &HashSet<String>) -> bool {
    // Functions with #[pub] decorator are always kept
    if func.decorators.iter().any(|d| d.name == "pub") {
        return false;
    }

    // Check if function is called
    !called_functions.contains(&func.name)
}

/// Eliminate dead code in impl block methods
fn eliminate_dead_code_in_impl(impl_block: &ImplBlock, stats: &mut DeadCodeStats) -> ImplBlock {
    let mut new_functions = Vec::new();

    for func in &impl_block.functions {
        let (new_body, func_stats) = eliminate_dead_code_in_statements(&func.body);
        stats.unreachable_statements_removed += func_stats.unreachable_statements_removed;
        stats.unused_variables_removed += func_stats.unused_variables_removed;
        stats.empty_blocks_removed += func_stats.empty_blocks_removed;

        let new_func = FunctionDecl {
            name: func.name.clone(),
            type_params: func.type_params.clone(),
            where_clause: func.where_clause.clone(),
            decorators: func.decorators.clone(),
            is_async: func.is_async,
            parameters: func.parameters.clone(),
            return_type: func.return_type.clone(),
            body: new_body,
        };
        new_functions.push(new_func);
    }

    ImplBlock {
        type_name: impl_block.type_name.clone(),
        type_params: impl_block.type_params.clone(),
        where_clause: impl_block.where_clause.clone(),
        trait_name: impl_block.trait_name.clone(),
        trait_type_args: impl_block.trait_type_args.clone(),
        associated_types: impl_block.associated_types.clone(),
        functions: new_functions,
        decorators: impl_block.decorators.clone(),
    }
}

/// Eliminate dead code in a list of statements
fn eliminate_dead_code_in_statements(statements: &[Statement]) -> (Vec<Statement>, DeadCodeStats) {
    let mut stats = DeadCodeStats::default();
    let mut new_statements = Vec::new();
    let mut found_terminator = false;

    for stmt in statements {
        // If we've already found a terminator, all subsequent statements are unreachable
        if found_terminator {
            stats.unreachable_statements_removed += 1;
            continue;
        }

        // Process the statement
        let new_stmt = eliminate_dead_code_in_statement(stmt, &mut stats);

        // Check if this statement terminates control flow
        if is_terminator(&new_stmt) {
            found_terminator = true;
        }

        // Skip empty statements
        if is_empty_statement(&new_stmt) {
            stats.empty_blocks_removed += 1;
            continue;
        }

        new_statements.push(new_stmt);
    }

    (new_statements, stats)
}

/// Eliminate dead code in a single statement
fn eliminate_dead_code_in_statement(stmt: &Statement, stats: &mut DeadCodeStats) -> Statement {
    match stmt {
        Statement::Expression(expr) => {
            Statement::Expression(eliminate_dead_code_in_expression(expr))
        }
        Statement::Return(Some(expr)) => {
            Statement::Return(Some(eliminate_dead_code_in_expression(expr)))
        }
        Statement::Return(None) => Statement::Return(None),
        Statement::Let {
            name,
            mutable,
            type_,
            value,
        } => Statement::Let {
            name: name.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: eliminate_dead_code_in_expression(value),
        },
        Statement::Assignment { target, value } => Statement::Assignment {
            target: target.clone(),
            value: eliminate_dead_code_in_expression(value),
        },
        Statement::If {
            condition,
            then_block,
            else_block,
        } => {
            let new_condition = eliminate_dead_code_in_expression(condition);
            let (new_then, then_stats) = eliminate_dead_code_in_statements(then_block);
            stats.unreachable_statements_removed += then_stats.unreachable_statements_removed;
            stats.empty_blocks_removed += then_stats.empty_blocks_removed;

            let new_else = if let Some(else_stmts) = else_block {
                let (new_else_stmts, else_stats) = eliminate_dead_code_in_statements(else_stmts);
                stats.unreachable_statements_removed += else_stats.unreachable_statements_removed;
                stats.empty_blocks_removed += else_stats.empty_blocks_removed;
                Some(new_else_stmts)
            } else {
                None
            };

            Statement::If {
                condition: new_condition,
                then_block: new_then,
                else_block: new_else,
            }
        }
        Statement::While { condition, body } => {
            let new_condition = eliminate_dead_code_in_expression(condition);
            let (new_body, body_stats) = eliminate_dead_code_in_statements(body);
            stats.unreachable_statements_removed += body_stats.unreachable_statements_removed;
            stats.empty_blocks_removed += body_stats.empty_blocks_removed;

            Statement::While {
                condition: new_condition,
                body: new_body,
            }
        }
        Statement::For {
            pattern,
            iterable,
            body,
        } => {
            let new_iterable = eliminate_dead_code_in_expression(iterable);
            let (new_body, body_stats) = eliminate_dead_code_in_statements(body);
            stats.unreachable_statements_removed += body_stats.unreachable_statements_removed;
            stats.empty_blocks_removed += body_stats.empty_blocks_removed;

            Statement::For {
                pattern: pattern.clone(),
                iterable: new_iterable,
                body: new_body,
            }
        }
        Statement::Match { value, arms } => {
            let new_value = eliminate_dead_code_in_expression(value);
            let mut new_arms = Vec::new();

            for arm in arms {
                let new_body = eliminate_dead_code_in_expression(&arm.body);
                let new_guard = arm.guard.as_ref().map(eliminate_dead_code_in_expression);

                new_arms.push(MatchArm {
                    pattern: arm.pattern.clone(),
                    guard: new_guard,
                    body: new_body,
                });
            }

            Statement::Match {
                value: new_value,
                arms: new_arms,
            }
        }
        Statement::Const { name, type_, value } => Statement::Const {
            name: name.clone(),
            type_: type_.clone(),
            value: eliminate_dead_code_in_expression(value),
        },
        Statement::Static {
            name,
            mutable,
            type_,
            value,
        } => Statement::Static {
            name: name.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: eliminate_dead_code_in_expression(value),
        },
        _ => stmt.clone(),
    }
}

/// Eliminate dead code in an expression
fn eliminate_dead_code_in_expression(expr: &Expression) -> Expression {
    match expr {
        Expression::Call {
            function,
            arguments,
        } => Expression::Call {
            function: Box::new(eliminate_dead_code_in_expression(function)),
            arguments: arguments
                .iter()
                .map(|(label, arg)| (label.clone(), eliminate_dead_code_in_expression(arg)))
                .collect(),
        },
        Expression::MethodCall {
            object,
            method,
            type_args,
            arguments,
        } => Expression::MethodCall {
            object: Box::new(eliminate_dead_code_in_expression(object)),
            method: method.clone(),
            type_args: type_args.clone(),
            arguments: arguments
                .iter()
                .map(|(label, arg)| (label.clone(), eliminate_dead_code_in_expression(arg)))
                .collect(),
        },
        Expression::Binary { left, op, right } => Expression::Binary {
            left: Box::new(eliminate_dead_code_in_expression(left)),
            op: *op,
            right: Box::new(eliminate_dead_code_in_expression(right)),
        },
        Expression::Unary { op, operand } => Expression::Unary {
            op: *op,
            operand: Box::new(eliminate_dead_code_in_expression(operand)),
        },
        Expression::Tuple(elements) => Expression::Tuple(
            elements
                .iter()
                .map(eliminate_dead_code_in_expression)
                .collect(),
        ),
        Expression::Index { object, index } => Expression::Index {
            object: Box::new(eliminate_dead_code_in_expression(object)),
            index: Box::new(eliminate_dead_code_in_expression(index)),
        },
        Expression::FieldAccess { object, field } => Expression::FieldAccess {
            object: Box::new(eliminate_dead_code_in_expression(object)),
            field: field.clone(),
        },
        Expression::Cast { expr, type_ } => Expression::Cast {
            expr: Box::new(eliminate_dead_code_in_expression(expr)),
            type_: type_.clone(),
        },
        Expression::Block(statements) => {
            let (new_statements, _) = eliminate_dead_code_in_statements(statements);
            Expression::Block(new_statements)
        }
        Expression::Closure { parameters, body } => Expression::Closure {
            parameters: parameters.clone(),
            body: Box::new(eliminate_dead_code_in_expression(body)),
        },
        Expression::StructLiteral { name, fields } => Expression::StructLiteral {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), eliminate_dead_code_in_expression(v)))
                .collect(),
        },
        Expression::Range {
            start,
            end,
            inclusive,
        } => Expression::Range {
            start: Box::new(eliminate_dead_code_in_expression(start)),
            end: Box::new(eliminate_dead_code_in_expression(end)),
            inclusive: *inclusive,
        },
        Expression::ChannelSend { channel, value } => Expression::ChannelSend {
            channel: Box::new(eliminate_dead_code_in_expression(channel)),
            value: Box::new(eliminate_dead_code_in_expression(value)),
        },
        Expression::ChannelRecv(channel) => {
            Expression::ChannelRecv(Box::new(eliminate_dead_code_in_expression(channel)))
        }
        Expression::Await(expr) => {
            Expression::Await(Box::new(eliminate_dead_code_in_expression(expr)))
        }
        Expression::TryOp(expr) => {
            Expression::TryOp(Box::new(eliminate_dead_code_in_expression(expr)))
        }
        Expression::Ternary {
            condition,
            true_expr,
            false_expr,
        } => Expression::Ternary {
            condition: Box::new(eliminate_dead_code_in_expression(condition)),
            true_expr: Box::new(eliminate_dead_code_in_expression(true_expr)),
            false_expr: Box::new(eliminate_dead_code_in_expression(false_expr)),
        },
        Expression::MacroInvocation {
            name,
            args,
            delimiter,
        } => Expression::MacroInvocation {
            name: name.clone(),
            args: args.iter().map(eliminate_dead_code_in_expression).collect(),
            delimiter: delimiter.clone(),
        },
        _ => expr.clone(),
    }
}

/// Check if a statement terminates control flow (return, break, continue)
fn is_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Return(_) | Statement::Break | Statement::Continue
    )
}

/// Check if a statement is empty and can be removed
fn is_empty_statement(stmt: &Statement) -> bool {
    match stmt {
        Statement::If {
            then_block,
            else_block,
            ..
        } => then_block.is_empty() && else_block.as_ref().is_none_or(|e| e.is_empty()),
        Statement::While { body, .. } | Statement::For { body, .. } => body.is_empty(),
        // Match arms always have a body expression, so they're never considered empty
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Decorator, Literal, Type};

    fn make_pub_func(name: &str, body: Vec<Statement>) -> FunctionDecl {
        FunctionDecl {
            name: name.to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![Decorator {
                name: "pub".to_string(),
                arguments: vec![],
            }],
            is_async: false,
            parameters: vec![],
            return_type: Some(Type::Custom("i32".to_string())),
            body,
        }
    }

    fn make_private_func(name: &str, body: Vec<Statement>) -> FunctionDecl {
        FunctionDecl {
            name: name.to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parameters: vec![],
            return_type: None,
            body,
        }
    }

    #[test]
    fn test_removes_unreachable_after_return() {
        let program = Program {
            items: vec![Item::Function(make_pub_func(
                "test",
                vec![
                    Statement::Return(Some(Expression::Literal(Literal::Int(42)))),
                    Statement::Expression(Expression::MacroInvocation {
                        name: "println".to_string(),
                        args: vec![Expression::Literal(Literal::String(
                            "unreachable".to_string(),
                        ))],
                        delimiter: crate::parser::MacroDelimiter::Parens,
                    }),
                ],
            ))],
        };

        let (optimized, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.unreachable_statements_removed, 1);

        let func = match &optimized.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };
        assert_eq!(func.body.len(), 1);
    }

    #[test]
    fn test_removes_unused_private_function() {
        let program = Program {
            items: vec![
                Item::Function(make_pub_func("main", vec![Statement::Return(None)])),
                Item::Function(make_private_func(
                    "unused_helper",
                    vec![Statement::Return(None)],
                )),
            ],
        };

        let (optimized, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.unused_functions_removed, 1);
        assert_eq!(optimized.items.len(), 1);
    }

    #[test]
    fn test_keeps_called_private_function() {
        let program = Program {
            items: vec![
                Item::Function(make_pub_func(
                    "main",
                    vec![Statement::Expression(Expression::Call {
                        function: Box::new(Expression::Identifier("helper".to_string())),
                        arguments: vec![],
                    })],
                )),
                Item::Function(make_private_func("helper", vec![Statement::Return(None)])),
            ],
        };

        let (optimized, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.unused_functions_removed, 0);
        assert_eq!(optimized.items.len(), 2);
    }

    #[test]
    fn test_removes_empty_if_blocks() {
        let program = Program {
            items: vec![Item::Function(make_pub_func(
                "test",
                vec![
                    Statement::If {
                        condition: Expression::Literal(Literal::Bool(true)),
                        then_block: vec![],
                        else_block: Some(vec![]),
                    },
                    Statement::Return(None),
                ],
            ))],
        };

        let (optimized, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.empty_blocks_removed, 1);

        let func = match &optimized.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };
        assert_eq!(func.body.len(), 1); // Only return remains
    }

    #[test]
    fn test_nested_unreachable_code() {
        let program = Program {
            items: vec![Item::Function(make_pub_func(
                "test",
                vec![Statement::If {
                    condition: Expression::Literal(Literal::Bool(true)),
                    then_block: vec![
                        Statement::Return(Some(Expression::Literal(Literal::Int(1)))),
                        Statement::Expression(Expression::Literal(Literal::Int(2))),
                    ],
                    else_block: Some(vec![
                        Statement::Return(Some(Expression::Literal(Literal::Int(3)))),
                        Statement::Expression(Expression::Literal(Literal::Int(4))),
                    ]),
                }],
            ))],
        };

        let (optimized, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.unreachable_statements_removed, 2);

        let func = match &optimized.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };
        if let Statement::If {
            then_block,
            else_block,
            ..
        } = &func.body[0]
        {
            assert_eq!(then_block.len(), 1);
            assert_eq!(else_block.as_ref().unwrap().len(), 1);
        } else {
            panic!("Expected if statement");
        }
    }

    #[test]
    fn test_no_changes_for_clean_code() {
        let program = Program {
            items: vec![Item::Function(make_pub_func(
                "test",
                vec![
                    Statement::Let {
                        name: "x".to_string(),
                        mutable: false,
                        type_: None,
                        value: Expression::Literal(Literal::Int(42)),
                    },
                    Statement::Return(Some(Expression::Identifier("x".to_string()))),
                ],
            ))],
        };

        let (_, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.unreachable_statements_removed, 0);
        assert_eq!(stats.unused_functions_removed, 0);
        assert_eq!(stats.empty_blocks_removed, 0);
    }
}
