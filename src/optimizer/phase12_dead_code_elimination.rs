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
pub fn eliminate_dead_code<'ast>(
    program: &Program<'ast>,
    optimizer: &crate::optimizer::Optimizer,
) -> (Program<'ast>, DeadCodeStats) {
    let mut stats = DeadCodeStats::default();

    // Step 1: Find all called functions (used to identify unused functions)
    let called_functions = find_called_functions(program);

    // Step 2: Process all items, removing dead code
    let mut new_items = Vec::new();
    for item in &program.items {
        match item {
            Item::Function {
                decl: func,
                location,
            } => {
                // Check if function is unused (private and never called)
                if is_unused_function(func, &called_functions) {
                    stats.unused_functions_removed += 1;
                    continue; // Skip this function
                }

                // Process function body to remove dead code
                let (new_body, func_stats) = eliminate_dead_code_in_statements(&func.body, optimizer);
                stats.unreachable_statements_removed += func_stats.unreachable_statements_removed;
                stats.unused_variables_removed += func_stats.unused_variables_removed;
                stats.empty_blocks_removed += func_stats.empty_blocks_removed;

                let new_func = FunctionDecl {
                    name: func.name.clone(),
                    is_pub: func.is_pub,
                    is_extern: func.is_extern,
                    type_params: func.type_params.clone(),
                    where_clause: func.where_clause.clone(),
                    decorators: func.decorators.clone(),
                    is_async: func.is_async,
                    parameters: func.parameters.clone(),
                    return_type: func.return_type.clone(),
                    body: new_body,
                    parent_type: func.parent_type.clone(),
                    doc_comment: func.doc_comment.clone(),
                };
                new_items.push(Item::Function {
                    decl: new_func,
                    location: location.clone(),
                });
            }
            Item::Impl {
                block: impl_block,
                location,
            } => {
                // Process impl block methods
                let new_impl = eliminate_dead_code_in_impl(impl_block, &mut stats, optimizer);
                new_items.push(Item::Impl {
                    block: new_impl,
                    location: location.clone(),
                });
            }
            Item::Static {
                name,
                mutable,
                type_,
                value,
                location,
            } => {
                // Process static initializers
                let new_value = eliminate_dead_code_in_expression(value, optimizer);
                new_items.push(Item::Static {
                    name: name.clone(),
                    mutable: *mutable,
                    type_: type_.clone(),
                    value: new_value,
                    location: location.clone(),
                });
            }
            Item::Const {
                name,
                type_,
                value,
                location,
            } => {
                // Process const initializers
                let new_value = eliminate_dead_code_in_expression(value, optimizer);
                new_items.push(Item::Const {
                    name: name.clone(),
                    type_: type_.clone(),
                    value: new_value,
                    location: location.clone(),
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
fn find_calls_in_statements<'ast>(statements: &[&'ast Statement<'ast>], called: &mut HashSet<String>) {
    for stmt in statements {
        find_calls_in_statement(stmt, called);
    }
}

/// Find function calls in a statement
fn find_calls_in_statement<'ast>(stmt: &'ast Statement<'ast>, called: &mut HashSet<String>) {
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
fn find_calls_in_expression<'ast>(expr: &'ast Expression<'ast>, called: &mut HashSet<String>) {
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
fn is_unused_function(func: &FunctionDecl, called_functions: &HashSet<String>) -> bool {
    // Functions with #[pub] decorator are always kept
    if func.decorators.iter().any(|d| d.name == "pub") {
        return false;
    }

    // Check if function is called
    !called_functions.contains(&func.name)
}

/// Eliminate dead code in impl block methods
fn eliminate_dead_code_in_impl<'ast>(
    impl_block: &ImplBlock<'ast>,
    stats: &mut DeadCodeStats,
    optimizer: &crate::optimizer::Optimizer,
) -> ImplBlock<'ast> {
    let mut new_functions = Vec::new();

    for func in &impl_block.functions {
        let (new_body, func_stats) = eliminate_dead_code_in_statements(&func.body, optimizer);
        stats.unreachable_statements_removed += func_stats.unreachable_statements_removed;
        stats.unused_variables_removed += func_stats.unused_variables_removed;
        stats.empty_blocks_removed += func_stats.empty_blocks_removed;

        let new_func = FunctionDecl {
            name: func.name.clone(),
            is_pub: func.is_pub,
            is_extern: func.is_extern,
            type_params: func.type_params.clone(),
            where_clause: func.where_clause.clone(),
            decorators: func.decorators.clone(),
            is_async: func.is_async,
            parameters: func.parameters.clone(),
            return_type: func.return_type.clone(),
            body: new_body,
            parent_type: func.parent_type.clone(),
            doc_comment: func.doc_comment.clone(),
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
fn eliminate_dead_code_in_statements<'ast>(
    statements: &[&'ast Statement<'ast>],
    optimizer: &crate::optimizer::Optimizer,
) -> (Vec<&'ast Statement<'ast>>, DeadCodeStats) {
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
        let new_stmt = eliminate_dead_code_in_statement(stmt, &mut stats, optimizer);

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
fn eliminate_dead_code_in_statement<'ast>(
    stmt: &'ast Statement<'ast>,
    stats: &mut DeadCodeStats,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Statement<'ast> {
    match stmt {
        Statement::Expression { expr, location } => optimizer.alloc_stmt(Statement::Expression {
            expr: eliminate_dead_code_in_expression(expr, optimizer),
            location: location.clone(),
        }),
        Statement::Return {
            value: Some(expr),
            location,
        } => optimizer.alloc_stmt(Statement::Return {
            value: Some(eliminate_dead_code_in_expression(expr, optimizer)),
            location: location.clone(),
        }),
        Statement::Return {
            value: None,
            location,
        } => optimizer.alloc_stmt(Statement::Return {
            value: None,
            location: location.clone(),
        }),
        Statement::Let {
            pattern,
            mutable,
            type_,
            value,
            else_block,
            location,
        } => optimizer.alloc_stmt(Statement::Let {
            pattern: pattern.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: eliminate_dead_code_in_expression(value, optimizer),
            else_block: else_block.as_ref().map(|stmts| {
                stmts
                    .iter()
                    .map(|s| eliminate_dead_code_in_statement(s, stats, optimizer))
                    .collect()
            }),
            location: location.clone(),
        }),
        Statement::Assignment {
            target,
            value,
            compound_op,
            location,
        } => optimizer.alloc_stmt(Statement::Assignment {
            target: target.clone(),
            value: eliminate_dead_code_in_expression(value, optimizer),
            compound_op: *compound_op,
            location: location.clone(),
        }),
        Statement::If {
            condition,
            then_block,
            else_block,
            location,
        } => {
            let new_condition = eliminate_dead_code_in_expression(condition, optimizer);
            let (new_then, then_stats) = eliminate_dead_code_in_statements(then_block, optimizer);
            stats.unreachable_statements_removed += then_stats.unreachable_statements_removed;
            stats.empty_blocks_removed += then_stats.empty_blocks_removed;

            let new_else = if let Some(else_stmts) = else_block {
                let (new_else_stmts, else_stats) = eliminate_dead_code_in_statements(else_stmts, optimizer);
                stats.unreachable_statements_removed += else_stats.unreachable_statements_removed;
                stats.empty_blocks_removed += else_stats.empty_blocks_removed;
                Some(new_else_stmts)
            } else {
                None
            };

            optimizer.alloc_stmt(Statement::If {
                condition: new_condition,
                then_block: new_then,
                else_block: new_else,
                location: location.clone(),
            })
        }
        Statement::While {
            condition,
            body,
            location,
        } => {
            let new_condition = eliminate_dead_code_in_expression(condition, optimizer);
            let (new_body, body_stats) = eliminate_dead_code_in_statements(body, optimizer);
            stats.unreachable_statements_removed += body_stats.unreachable_statements_removed;
            stats.empty_blocks_removed += body_stats.empty_blocks_removed;

            optimizer.alloc_stmt(Statement::While {
                condition: new_condition,
                body: new_body,
                location: location.clone(),
            })
        }
        Statement::For {
            pattern,
            iterable,
            body,
            location,
        } => {
            let new_iterable = eliminate_dead_code_in_expression(iterable, optimizer);
            let (new_body, body_stats) = eliminate_dead_code_in_statements(body, optimizer);
            stats.unreachable_statements_removed += body_stats.unreachable_statements_removed;
            stats.empty_blocks_removed += body_stats.empty_blocks_removed;

            optimizer.alloc_stmt(Statement::For {
                pattern: pattern.clone(),
                iterable: new_iterable,
                body: new_body,
                location: location.clone(),
            })
        }
        Statement::Match {
            value,
            arms,
            location,
        } => {
            let new_value = eliminate_dead_code_in_expression(value, optimizer);
            let mut new_arms = Vec::new();

            for arm in arms {
                let new_body = eliminate_dead_code_in_expression(&arm.body, optimizer);
                let new_guard = arm.guard.as_ref().map(|g| eliminate_dead_code_in_expression(g, optimizer));

                new_arms.push(MatchArm {
                    pattern: arm.pattern.clone(),
                    guard: new_guard,
                    body: new_body,
                    location: arm.location.clone(),
                });
            }

            optimizer.alloc_stmt(Statement::Match {
                value: new_value,
                arms: new_arms,
                location: location.clone(),
            })
        }
        Statement::Const {
            name,
            type_,
            value,
            location,
        } => optimizer.alloc_stmt(Statement::Const {
            name: name.clone(),
            type_: type_.clone(),
            value: eliminate_dead_code_in_expression(value, optimizer),
            location: location.clone(),
        }),
        Statement::Static {
            name,
            mutable,
            type_,
            value,
            location,
        } => optimizer.alloc_stmt(Statement::Static {
            name: name.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: eliminate_dead_code_in_expression(value, optimizer),
            location: location.clone(),
        }),
        _ => stmt,
    }
}

/// Eliminate dead code in an expression
fn eliminate_dead_code_in_expression<'a, 'ast>(
    expr: &'a Expression<'a>,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Expression<'ast> {
    match expr {
        Expression::Call {
            function,
            arguments,
            location,
        } => optimizer.alloc_expr(Expression::Call {
            function: eliminate_dead_code_in_expression(function, optimizer),
            arguments: arguments
                .iter()
                .map(|(label, arg)| (label.clone(), eliminate_dead_code_in_expression(arg, optimizer)))
                .collect(),
            location: location.clone(),
        }),
        Expression::MethodCall {
            object,
            method,
            type_args,
            arguments,
            location,
        } => optimizer.alloc_expr(Expression::MethodCall {
            object: eliminate_dead_code_in_expression(object, optimizer),
            method: method.clone(),
            type_args: type_args.clone(),
            arguments: arguments
                .iter()
                .map(|(label, arg)| (label.clone(), eliminate_dead_code_in_expression(arg, optimizer)))
                .collect(),
            location: location.clone(),
        }),
        Expression::Binary {
            left,
            op,
            right,
            location,
        } => optimizer.alloc_expr(Expression::Binary {
            left: eliminate_dead_code_in_expression(left, optimizer),
            op: *op,
            right: eliminate_dead_code_in_expression(right, optimizer),
            location: location.clone(),
        }),
        Expression::Unary {
            op,
            operand,
            location,
        } => optimizer.alloc_expr(Expression::Unary {
            op: *op,
            operand: eliminate_dead_code_in_expression(operand, optimizer),
            location: location.clone(),
        }),
        Expression::Tuple { elements, location } => optimizer.alloc_expr(Expression::Tuple {
            elements: elements
                .iter()
                .map(|e| eliminate_dead_code_in_expression(e, optimizer))
                .collect(),
            location: location.clone(),
        }),
        Expression::Index {
            object,
            index,
            location,
        } => optimizer.alloc_expr(Expression::Index {
            object: eliminate_dead_code_in_expression(object, optimizer),
            index: eliminate_dead_code_in_expression(index, optimizer),
            location: location.clone(),
        }),
        Expression::FieldAccess {
            object,
            field,
            location,
        } => optimizer.alloc_expr(Expression::FieldAccess {
            object: eliminate_dead_code_in_expression(object, optimizer),
            field: field.clone(),
            location: location.clone(),
        }),
        Expression::Cast {
            expr,
            type_,
            location,
        } => optimizer.alloc_expr(Expression::Cast {
            expr: eliminate_dead_code_in_expression(expr, optimizer),
            type_: type_.clone(),
            location: location.clone(),
        }),
        Expression::Block {
            statements,
            location,
        } => {
            let (new_statements, _) = eliminate_dead_code_in_statements(statements, optimizer);
            optimizer.alloc_expr(Expression::Block {
                statements: new_statements,
                location: location.clone(),
            })
        }
        Expression::Closure {
            parameters,
            body,
            location,
        } => optimizer.alloc_expr(Expression::Closure {
            parameters: parameters.clone(),
            body: eliminate_dead_code_in_expression(body, optimizer),
            location: location.clone(),
        }),
        Expression::StructLiteral {
            name,
            fields,
            location,
        } => optimizer.alloc_expr(Expression::StructLiteral {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), eliminate_dead_code_in_expression(v, optimizer)))
                .collect(),
            location: location.clone(),
        }),
        Expression::Range {
            start,
            end,
            inclusive,
            location,
        } => optimizer.alloc_expr(Expression::Range {
            start: eliminate_dead_code_in_expression(start, optimizer),
            end: eliminate_dead_code_in_expression(end, optimizer),
            inclusive: *inclusive,
            location: location.clone(),
        }),
        Expression::ChannelSend {
            channel,
            value,
            location,
        } => optimizer.alloc_expr(Expression::ChannelSend {
            channel: eliminate_dead_code_in_expression(channel, optimizer),
            value: eliminate_dead_code_in_expression(value, optimizer),
            location: location.clone(),
        }),
        Expression::ChannelRecv { channel, location } => optimizer.alloc_expr(Expression::ChannelRecv {
            channel: eliminate_dead_code_in_expression(channel, optimizer),
            location: location.clone(),
        }),
        Expression::Await { expr, location } => optimizer.alloc_expr(Expression::Await {
            expr: eliminate_dead_code_in_expression(expr, optimizer),
            location: location.clone(),
        }),
        Expression::TryOp { expr, location } => optimizer.alloc_expr(Expression::TryOp {
            expr: eliminate_dead_code_in_expression(expr, optimizer),
            location: location.clone(),
        }),
        Expression::MacroInvocation {
            name,
            args,
            delimiter,
            location,
        } => optimizer.alloc_expr(Expression::MacroInvocation {
            name: name.clone(),
            args: args.iter().map(|a| eliminate_dead_code_in_expression(a, optimizer)).collect(),
            delimiter: *delimiter,
            location: location.clone(),
        }),
        _ => expr,
    }
}

/// Check if a statement terminates control flow (return, break, continue)
fn is_terminator(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Return { .. } | Statement::Break { .. } | Statement::Continue { .. }
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
    use crate::parser::{Decorator, Literal, Pattern, Type};

    fn make_pub_func(name: &str, body: Vec<Statement>) -> FunctionDecl {
        FunctionDecl {
            is_pub: false,
            is_extern: false,
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
            parent_type: None,
            doc_comment: None,
        }
    }

    fn make_private_func(name: &str, body: Vec<Statement>) -> FunctionDecl {
        FunctionDecl {
            is_pub: false,
            is_extern: false,
            name: name.to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parameters: vec![],
            return_type: None,
            body,
            parent_type: None,
            doc_comment: None,
        }
    }

    #[test]
    fn test_removes_unreachable_after_return() {
        let program = Program {
            items: vec![Item::Function {
                decl: make_pub_func(
                    "test",
                    vec![
                        Statement::Return {
                            value: Some(Expression::Literal {
                                value: Literal::Int(42),
                                location: None,
                            }),
                            location: None,
                        },
                        Statement::Expression {
                            expr: Expression::MacroInvocation {
                                name: "println".to_string(),
                                args: vec![Expression::Literal {
                                    value: Literal::String("unreachable".to_string()),
                                    location: None,
                                }],
                                delimiter: crate::parser::MacroDelimiter::Parens,
                                location: None,
                            },
                            location: None,
                        },
                    ],
                ),
                location: None,
            }],
        };

        let (optimized, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.unreachable_statements_removed, 1);

        let func = match &optimized.items[0] {
            Item::Function { decl: f, .. } => f,
            _ => panic!("Expected function"),
        };
        assert_eq!(func.body.len(), 1);
    }

    #[test]
    fn test_removes_unused_private_function() {
        let program = Program {
            items: vec![
                Item::Function {
                    decl: make_pub_func(
                        "main",
                        vec![Statement::Return {
                            value: None,
                            location: None,
                        }],
                    ),
                    location: None,
                },
                Item::Function {
                    decl: make_private_func(
                        "unused_helper",
                        vec![Statement::Return {
                            value: None,
                            location: None,
                        }],
                    ),
                    location: None,
                },
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
                Item::Function {
                    decl: make_pub_func(
                        "main",
                        vec![Statement::Expression {
                            expr: Expression::Call {
                                function: Box::new(Expression::Identifier {
                                    name: "helper".to_string(),
                                    location: None,
                                }),
                                arguments: vec![],
                                location: None,
                            },
                            location: None,
                        }],
                    ),
                    location: None,
                },
                Item::Function {
                    decl: make_private_func(
                        "helper",
                        vec![Statement::Return {
                            value: None,
                            location: None,
                        }],
                    ),
                    location: None,
                },
            ],
        };

        let (optimized, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.unused_functions_removed, 0);
        assert_eq!(optimized.items.len(), 2);
    }

    #[test]
    fn test_removes_empty_if_blocks() {
        let program = Program {
            items: vec![Item::Function {
                decl: make_pub_func(
                    "test",
                    vec![
                        Statement::If {
                            condition: Expression::Literal {
                                value: Literal::Bool(true),
                                location: None,
                            },
                            then_block: vec![],
                            else_block: Some(vec![]),
                            location: None,
                        },
                        Statement::Return {
                            value: None,
                            location: None,
                        },
                    ],
                ),
                location: None,
            }],
        };

        let (optimized, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.empty_blocks_removed, 1);

        let func = match &optimized.items[0] {
            Item::Function { decl: f, .. } => f,
            _ => panic!("Expected function"),
        };
        assert_eq!(func.body.len(), 1); // Only return remains
    }

    #[test]
    fn test_nested_unreachable_code() {
        let program = Program {
            items: vec![Item::Function {
                decl: make_pub_func(
                    "test",
                    vec![Statement::If {
                        condition: Expression::Literal {
                            value: Literal::Bool(true),
                            location: None,
                        },
                        then_block: vec![
                            Statement::Return {
                                value: Some(Expression::Literal {
                                    value: Literal::Int(1),
                                    location: None,
                                }),
                                location: None,
                            },
                            Statement::Expression {
                                expr: Expression::Literal {
                                    value: Literal::Int(2),
                                    location: None,
                                },
                                location: None,
                            },
                        ],
                        else_block: Some(vec![
                            Statement::Return {
                                value: Some(Expression::Literal {
                                    value: Literal::Int(3),
                                    location: None,
                                }),
                                location: None,
                            },
                            Statement::Expression {
                                expr: Expression::Literal {
                                    value: Literal::Int(4),
                                    location: None,
                                },
                                location: None,
                            },
                        ]),
                        location: None,
                    }],
                ),
                location: None,
            }],
        };

        let (optimized, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.unreachable_statements_removed, 2);

        let func = match &optimized.items[0] {
            Item::Function { decl, .. } => decl,
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
            items: vec![Item::Function {
                decl: make_pub_func(
                    "test",
                    vec![
                        Statement::Let {
                            pattern: Pattern::Identifier("x".to_string()),
                            mutable: false,
                            type_: None,
                            value: Expression::Literal {
                                value: Literal::Int(42),
                                location: None,
                            },
                            else_block: None,
                            location: None,
                        },
                        Statement::Return {
                            value: Some(Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            }),
                            location: None,
                        },
                    ],
                ),
                location: None,
            }],
        };

        let (_, stats) = eliminate_dead_code(&program);
        assert_eq!(stats.unreachable_statements_removed, 0);
        assert_eq!(stats.unused_functions_removed, 0);
        assert_eq!(stats.empty_blocks_removed, 0);
    }
}
