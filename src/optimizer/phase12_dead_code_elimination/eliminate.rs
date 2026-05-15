//! Recursive dead-code removal in statements and expressions.

use crate::parser::{Expression, FunctionDecl, ImplBlock, MatchArm, Statement};

use super::{control_flow, DeadCodeStats};


/// Eliminate dead code in impl block methods
pub(super) fn eliminate_dead_code_in_impl<'ast>(
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
            return_decorators: func.return_decorators.clone(),
            body: new_body,
            parent_type: func.parent_type.clone(),
            impl_trait: func.impl_trait.clone(),
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
        is_extern: impl_block.is_extern,
    }
}

/// Eliminate dead code in a list of statements
pub(super) fn eliminate_dead_code_in_statements<'ast>(
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
        if control_flow::is_terminator(new_stmt) {
            found_terminator = true;
        }

        // Skip empty statements
        if control_flow::is_empty_statement(new_stmt) {
            stats.empty_blocks_removed += 1;
            continue;
        }

        new_statements.push(new_stmt);
    }

    (new_statements, stats)
}

/// Eliminate dead code in a single statement
pub(super) fn eliminate_dead_code_in_statement<'ast>(
    stmt: &'ast Statement<'ast>,
    stats: &mut DeadCodeStats,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Statement<'ast> {
    match stmt {
        Statement::Expression { expr, location } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Expression {
                expr: eliminate_dead_code_in_expression(expr, optimizer),
                location: location.clone(),
            })
        }),
        Statement::Return {
            value: Some(expr),
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Return {
                value: Some(eliminate_dead_code_in_expression(expr, optimizer)),
                location: location.clone(),
            })
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
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Let {
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
            })
        }),
        Statement::Assignment {
            target,
            value,
            compound_op,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Assignment {
                target,
                value: eliminate_dead_code_in_expression(value, optimizer),
                compound_op: *compound_op,
                location: location.clone(),
            })
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
                let (new_else_stmts, else_stats) =
                    eliminate_dead_code_in_statements(else_stmts, optimizer);
                stats.unreachable_statements_removed += else_stats.unreachable_statements_removed;
                stats.empty_blocks_removed += else_stats.empty_blocks_removed;
                Some(new_else_stmts)
            } else {
                None
            };

            optimizer.alloc_stmt(unsafe {
                std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::If {
                    condition: new_condition,
                    then_block: new_then,
                    else_block: new_else,
                    location: location.clone(),
                })
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

            optimizer.alloc_stmt(unsafe {
                std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::While {
                    condition: new_condition,
                    body: new_body,
                    location: location.clone(),
                })
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

            optimizer.alloc_stmt(unsafe {
                std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::For {
                    pattern: pattern.clone(),
                    iterable: new_iterable,
                    body: new_body,
                    location: location.clone(),
                })
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
                let new_body = eliminate_dead_code_in_expression(arm.body, optimizer);
                let new_guard = arm
                    .guard
                    .as_ref()
                    .map(|g| eliminate_dead_code_in_expression(g, optimizer));

                new_arms.push(MatchArm {
                    pattern: arm.pattern.clone(),
                    guard: new_guard,
                    body: new_body,
                });
            }

            optimizer.alloc_stmt(unsafe {
                std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Match {
                    value: new_value,
                    arms: new_arms,
                    location: location.clone(),
                })
            })
        }
        Statement::Const {
            name,
            type_,
            value,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Const {
                name: name.clone(),
                type_: type_.clone(),
                value: eliminate_dead_code_in_expression(value, optimizer),
                location: location.clone(),
            })
        }),
        Statement::Static {
            name,
            mutable,
            type_,
            value,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Static {
                name: name.clone(),
                mutable: *mutable,
                type_: type_.clone(),
                value: eliminate_dead_code_in_expression(value, optimizer),
                location: location.clone(),
            })
        }),
        _ => stmt,
    }
}

/// Eliminate dead code in an expression
pub(super) fn eliminate_dead_code_in_expression<'a: 'ast, 'ast>(
    expr: &'a Expression<'a>,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Expression<'ast> {
    match expr {
        Expression::Call {
            function,
            arguments,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Call {
                function: eliminate_dead_code_in_expression(function, optimizer),
                arguments: arguments
                    .iter()
                    .map(|(label, arg)| {
                        (
                            label.clone(),
                            eliminate_dead_code_in_expression(arg, optimizer),
                        )
                    })
                    .collect(),
                location: location.clone(),
            })
        }),
        Expression::MethodCall {
            object,
            method,
            type_args,
            arguments,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::MethodCall {
                object: eliminate_dead_code_in_expression(object, optimizer),
                method: method.clone(),
                type_args: type_args.clone(),
                arguments: arguments
                    .iter()
                    .map(|(label, arg)| {
                        (
                            label.clone(),
                            eliminate_dead_code_in_expression(arg, optimizer),
                        )
                    })
                    .collect(),
                location: location.clone(),
            })
        }),
        Expression::Binary {
            left,
            op,
            right,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Binary {
                left: eliminate_dead_code_in_expression(left, optimizer),
                op: *op,
                right: eliminate_dead_code_in_expression(right, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Unary {
            op,
            operand,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Unary {
                op: *op,
                operand: eliminate_dead_code_in_expression(operand, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Tuple { elements, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Tuple {
                elements: elements
                    .iter()
                    .map(|e| eliminate_dead_code_in_expression(e, optimizer))
                    .collect(),
                location: location.clone(),
            })
        }),
        Expression::Index {
            object,
            index,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Index {
                object: eliminate_dead_code_in_expression(object, optimizer),
                index: eliminate_dead_code_in_expression(index, optimizer),
                location: location.clone(),
            })
        }),
        Expression::FieldAccess {
            object,
            field,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::FieldAccess {
                object: eliminate_dead_code_in_expression(object, optimizer),
                field: field.clone(),
                location: location.clone(),
            })
        }),
        Expression::Cast {
            expr,
            type_,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Cast {
                expr: eliminate_dead_code_in_expression(expr, optimizer),
                type_: type_.clone(),
                location: location.clone(),
            })
        }),
        Expression::Block {
            statements,
            is_unsafe,
            location,
        } => {
            let (new_statements, _) = eliminate_dead_code_in_statements(statements, optimizer);
            optimizer.alloc_expr(unsafe {
                std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Block {
                    statements: new_statements,
                    is_unsafe: *is_unsafe,
                    location: location.clone(),
                })
            })
        }
        Expression::Closure {
            parameters,
            body,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Closure {
                parameters: parameters.clone(),
                body: eliminate_dead_code_in_expression(body, optimizer),
                location: location.clone(),
            })
        }),
        Expression::StructLiteral {
            name,
            fields,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::StructLiteral {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|(k, v)| (k.clone(), eliminate_dead_code_in_expression(v, optimizer)))
                    .collect(),
                location: location.clone(),
            })
        }),
        Expression::Range {
            start,
            end,
            inclusive,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Range {
                start: eliminate_dead_code_in_expression(start, optimizer),
                end: eliminate_dead_code_in_expression(end, optimizer),
                inclusive: *inclusive,
                location: location.clone(),
            })
        }),
        Expression::ChannelSend {
            channel,
            value,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::ChannelSend {
                channel: eliminate_dead_code_in_expression(channel, optimizer),
                value: eliminate_dead_code_in_expression(value, optimizer),
                location: location.clone(),
            })
        }),
        Expression::ChannelRecv { channel, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::ChannelRecv {
                channel: eliminate_dead_code_in_expression(channel, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Await { expr, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Await {
                expr: eliminate_dead_code_in_expression(expr, optimizer),
                location: location.clone(),
            })
        }),
        Expression::TryOp { expr, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::TryOp {
                expr: eliminate_dead_code_in_expression(expr, optimizer),
                location: location.clone(),
            })
        }),
        Expression::MacroInvocation {
            name,
            args,
            delimiter,
            is_repeat,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::MacroInvocation {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| eliminate_dead_code_in_expression(a, optimizer))
                    .collect(),
                delimiter: *delimiter,
                is_repeat: *is_repeat,
                location: location.clone(),
            })
        }),
        _ => expr,
    }
}
