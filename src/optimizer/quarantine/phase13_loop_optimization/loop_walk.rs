//! Recursive AST walk that applies loop optimizations.

use crate::parser::{Expression, Statement};

use super::loop_invariant_motion::hoist_loop_invariants;
use super::loop_transformations::{try_strength_reduction, try_unroll_loop};
use super::{LoopOptimizationConfig, LoopOptimizationStats};

/// Optimize loops in a list of statements
pub(super) fn optimize_loops_in_statements<'ast>(
    statements: &[&'ast Statement<'ast>],
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
    optimizer: &crate::optimizer::Optimizer,
) -> Vec<&'ast Statement<'ast>> {
    let mut result = Vec::new();

    for stmt in statements {
        match stmt {
            Statement::For {
                pattern,
                iterable,
                body,
                location,
            } => {
                // Extract simple identifier from pattern for optimization
                // Only optimize simple loops with a single identifier pattern
                if let crate::parser::Pattern::Identifier(variable) = pattern {
                    // Try to unroll the loop if it's a small constant range
                    if config.enable_unrolling {
                        if let Some(unrolled) =
                            try_unroll_loop(variable, iterable, body, config, stats, optimizer)
                        {
                            result.extend(unrolled);
                            stats.loops_unrolled += 1;
                            stats.loops_optimized += 1;
                            continue;
                        }
                    }

                    // Apply LICM if enabled
                    let optimized_body = if config.enable_licm {
                        let (hoisted, new_body) =
                            hoist_loop_invariants(body, variable, stats, optimizer);
                        let has_hoisted = !hoisted.is_empty();
                        result.extend(hoisted);
                        if has_hoisted {
                            stats.loops_optimized += 1;
                        }
                        new_body
                    } else {
                        body.clone()
                    };

                    // Recursively optimize the loop body
                    let final_body =
                        optimize_loops_in_statements(&optimized_body, config, stats, optimizer);

                    result.push(optimizer.alloc_stmt(unsafe {
                        std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::For {
                            pattern: pattern.clone(),
                            iterable: optimize_loops_in_expression(
                                iterable, config, stats, optimizer,
                            ),
                            body: final_body,
                            location: location.clone(),
                        })
                    }));
                } else {
                    // For complex patterns (tuples, etc.), just recursively optimize the body
                    let final_body = optimize_loops_in_statements(body, config, stats, optimizer);
                    result.push(optimizer.alloc_stmt(unsafe {
                        std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::For {
                            pattern: pattern.clone(),
                            iterable: optimize_loops_in_expression(
                                iterable, config, stats, optimizer,
                            ),
                            body: final_body,
                            location: location.clone(),
                        })
                    }));
                }
            }
            Statement::While {
                condition,
                body,
                location,
            } => {
                // Apply LICM if enabled
                let optimized_body = if config.enable_licm {
                    let (hoisted, new_body) = hoist_loop_invariants(body, "", stats, optimizer);
                    let has_hoisted = !hoisted.is_empty();
                    result.extend(hoisted);
                    if has_hoisted {
                        stats.loops_optimized += 1;
                    }
                    new_body
                } else {
                    body.clone()
                };

                // Recursively optimize the loop body
                let final_body =
                    optimize_loops_in_statements(&optimized_body, config, stats, optimizer);

                result.push(optimizer.alloc_stmt(unsafe {
                    std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::While {
                        condition: optimize_loops_in_expression(
                            condition, config, stats, optimizer,
                        ),
                        body: final_body,
                        location: location.clone(),
                    })
                }));
            }
            _ => result.push(optimize_loops_in_statement(stmt, config, stats, optimizer)),
        }
    }

    result
}

/// Optimize loops in a single statement
pub(super) fn optimize_loops_in_statement<'a: 'ast, 'ast>(
    stmt: &'a Statement<'a>,
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Statement<'ast> {
    match stmt {
        Statement::Expression { expr, location } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Expression {
                expr: optimize_loops_in_expression(expr, config, stats, optimizer),
                location: location.clone(),
            })
        }),
        Statement::Return {
            value: Some(expr),
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Return {
                value: Some(optimize_loops_in_expression(expr, config, stats, optimizer)),
                location: location.clone(),
            })
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
                value: optimize_loops_in_expression(value, config, stats, optimizer),
                else_block: else_block.as_ref().map(|stmts| {
                    stmts
                        .iter()
                        .map(|s| optimize_loops_in_statement(s, config, stats, optimizer))
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
                value: optimize_loops_in_expression(value, config, stats, optimizer),
                compound_op: *compound_op,
                location: location.clone(),
            })
        }),
        Statement::If {
            condition,
            then_block,
            else_block,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::If {
                condition: optimize_loops_in_expression(condition, config, stats, optimizer),
                then_block: optimize_loops_in_statements(then_block, config, stats, optimizer),
                else_block: else_block
                    .as_ref()
                    .map(|stmts| optimize_loops_in_statements(stmts, config, stats, optimizer)),
                location: location.clone(),
            })
        }),
        Statement::Match {
            value,
            arms,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Match {
                value: optimize_loops_in_expression(value, config, stats, optimizer),
                arms: arms
                    .iter()
                    .map(|arm| crate::parser::MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm
                            .guard
                            .as_ref()
                            .map(|g| optimize_loops_in_expression(g, config, stats, optimizer)),
                        body: optimize_loops_in_expression(arm.body, config, stats, optimizer),
                    })
                    .collect(),
                location: location.clone(),
            })
        }),
        _ => stmt,
    }
}

/// Optimize loops in an expression
pub(super) fn optimize_loops_in_expression<'a: 'ast, 'ast>(
    expr: &'a Expression<'a>,
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Expression<'ast> {
    match expr {
        Expression::Call {
            function,
            arguments,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Call {
                function: optimize_loops_in_expression(function, config, stats, optimizer),
                arguments: arguments
                    .iter()
                    .map(|(label, arg)| {
                        (
                            label.clone(),
                            optimize_loops_in_expression(arg, config, stats, optimizer),
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
                object: optimize_loops_in_expression(object, config, stats, optimizer),
                method: method.clone(),
                type_args: type_args.clone(),
                arguments: arguments
                    .iter()
                    .map(|(label, arg)| {
                        (
                            label.clone(),
                            optimize_loops_in_expression(arg, config, stats, optimizer),
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
        } => {
            // Apply strength reduction if enabled
            if config.enable_strength_reduction {
                if let Some(reduced) =
                    try_strength_reduction(left, op, right, config, stats, optimizer)
                {
                    return reduced;
                }
            }

            optimizer.alloc_expr(unsafe {
                std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Binary {
                    left: optimize_loops_in_expression(left, config, stats, optimizer),
                    op: *op,
                    right: optimize_loops_in_expression(right, config, stats, optimizer),
                    location: location.clone(),
                })
            })
        }
        Expression::Unary {
            op,
            operand,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Unary {
                op: *op,
                operand: optimize_loops_in_expression(operand, config, stats, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Block {
            statements,
            is_unsafe,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Block {
                statements: optimize_loops_in_statements(statements, config, stats, optimizer),
                is_unsafe: *is_unsafe,
                location: location.clone(),
            })
        }),
        Expression::Closure {
            parameters,
            body,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Closure {
                parameters: parameters.clone(),
                body: optimize_loops_in_expression(body, config, stats, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Index {
            object,
            index,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Index {
                object: optimize_loops_in_expression(object, config, stats, optimizer),
                index: optimize_loops_in_expression(index, config, stats, optimizer),
                location: location.clone(),
            })
        }),
        Expression::FieldAccess {
            object,
            field,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::FieldAccess {
                object: optimize_loops_in_expression(object, config, stats, optimizer),
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
                expr: optimize_loops_in_expression(expr, config, stats, optimizer),
                type_: type_.clone(),
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
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            optimize_loops_in_expression(v, config, stats, optimizer),
                        )
                    })
                    .collect(),
                location: location.clone(),
            })
        }),
        Expression::Tuple { elements, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Tuple {
                elements: elements
                    .iter()
                    .map(|e| optimize_loops_in_expression(e, config, stats, optimizer))
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
                start: optimize_loops_in_expression(start, config, stats, optimizer),
                end: optimize_loops_in_expression(end, config, stats, optimizer),
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
                channel: optimize_loops_in_expression(channel, config, stats, optimizer),
                value: optimize_loops_in_expression(value, config, stats, optimizer),
                location: location.clone(),
            })
        }),
        Expression::ChannelRecv { channel, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::ChannelRecv {
                channel: optimize_loops_in_expression(channel, config, stats, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Await { expr, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Await {
                expr: optimize_loops_in_expression(expr, config, stats, optimizer),
                location: location.clone(),
            })
        }),
        Expression::TryOp { expr, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::TryOp {
                expr: optimize_loops_in_expression(expr, config, stats, optimizer),
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
                    .map(|a| optimize_loops_in_expression(a, config, stats, optimizer))
                    .collect(),
                delimiter: *delimiter,
                is_repeat: *is_repeat,
                location: location.clone(),
            })
        }),
        _ => expr,
    }
}
