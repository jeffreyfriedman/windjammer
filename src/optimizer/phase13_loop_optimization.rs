//! Phase 13: Loop Optimization
//!
//! This optimization improves loop performance through several techniques:
//! - **Loop Invariant Code Motion (LICM)**: Moves loop-invariant computations outside the loop
//! - **Loop Unrolling**: Unrolls small loops to reduce overhead
//! - **Strength Reduction**: Replaces expensive operations with cheaper ones
//!
//! Example transformations:
//! ```text
//! // Before LICM:
//! for i in 0..100 {
//!     let x = expensive_call()  // Loop invariant
//!     process(i, x)
//! }
//!
//! // After LICM:
//! let x = expensive_call()
//! for i in 0..100 {
//!     process(i, x)
//! }
//!
//! // Before unrolling:
//! for i in 0..4 { array[i] = i }
//!
//! // After unrolling:
//! array[0] = 0; array[1] = 1; array[2] = 2; array[3] = 3;
//! ```

use crate::parser::{
    BinaryOp, Expression, FunctionDecl, ImplBlock, Item, Pattern, Program, Statement,
};
// No additional imports needed for Phase 13

/// Statistics about loop optimizations
#[derive(Debug, Default, Clone)]
pub struct LoopOptimizationStats {
    pub loops_optimized: usize,
    pub invariants_hoisted: usize,
    pub loops_unrolled: usize,
    pub strength_reductions: usize,
}

/// Configuration for loop optimization
#[derive(Debug, Clone)]
pub struct LoopOptimizationConfig {
    /// Enable loop invariant code motion
    pub enable_licm: bool,
    /// Enable loop unrolling
    pub enable_unrolling: bool,
    /// Maximum iteration count for loop unrolling (loops with more iterations won't be unrolled)
    pub max_unroll_iterations: usize,
    /// Enable strength reduction
    pub enable_strength_reduction: bool,
}

impl Default for LoopOptimizationConfig {
    fn default() -> Self {
        Self {
            enable_licm: true,
            enable_unrolling: true,
            max_unroll_iterations: 8,
            enable_strength_reduction: true,
        }
    }
}

/// Optimize loops in a program
pub fn optimize_loops<'ast>(program: &Program<'ast>, optimizer: &crate::optimizer::Optimizer) -> (Program<'ast>, LoopOptimizationStats) {
    optimize_loops_with_config(program, &LoopOptimizationConfig::default(), optimizer)
}

/// Optimize loops with custom configuration
pub fn optimize_loops_with_config<'ast>(
    program: &'ast Program<'ast>,
    config: &LoopOptimizationConfig,
    optimizer: &crate::optimizer::Optimizer,
) -> (Program<'ast>, LoopOptimizationStats) {
    let mut stats = LoopOptimizationStats::default();

    let new_items = program
        .items
        .iter()
        .map(|item| optimize_loops_in_item(item, config, &mut stats, optimizer))
        .collect();

    (Program { items: new_items }, stats)
}

/// Optimize loops in a single item
fn optimize_loops_in_item<'ast>(
    item: &'ast Item<'ast>,
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
    optimizer: &crate::optimizer::Optimizer,
) -> Item<'ast> {
    match item {
        Item::Function {
            decl: func,
            location,
        } => {
            let new_body = optimize_loops_in_statements(&func.body, config, stats, optimizer);
            Item::Function {
                decl: FunctionDecl {
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
                },
                location: location.clone(),
            }
        }
        Item::Impl {
            block: impl_block,
            location,
        } => {
            let new_functions = impl_block
                .functions
                .iter()
                .map(|func| FunctionDecl {
                    name: func.name.clone(),
                    is_pub: func.is_pub,
                    is_extern: func.is_extern,
                    type_params: func.type_params.clone(),
                    where_clause: func.where_clause.clone(),
                    decorators: func.decorators.clone(),
                    is_async: func.is_async,
                    parameters: func.parameters.clone(),
                    return_type: func.return_type.clone(),
                    body: optimize_loops_in_statements(&func.body, config, stats, optimizer),
                    parent_type: func.parent_type.clone(),
                    doc_comment: func.doc_comment.clone(),
                })
                .collect();

            Item::Impl {
                block: ImplBlock {
                    type_name: impl_block.type_name.clone(),
                    type_params: impl_block.type_params.clone(),
                    where_clause: impl_block.where_clause.clone(),
                    trait_name: impl_block.trait_name.clone(),
                    trait_type_args: impl_block.trait_type_args.clone(),
                    associated_types: impl_block.associated_types.clone(),
                    functions: new_functions,
                    decorators: impl_block.decorators.clone(),
                },
                location: location.clone(),
            }
        }
        Item::Static {
            name,
            mutable,
            type_,
            value,
            location,
        } => Item::Static {
            name: name.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: optimize_loops_in_expression(value, config, stats, optimizer),
            location: location.clone(),
        },
        Item::Const {
            name,
            type_,
            value,
            location,
        } => Item::Const {
            name: name.clone(),
            type_: type_.clone(),
            value: optimize_loops_in_expression(value, config, stats, optimizer),
            location: location.clone(),
        },
        _ => item.clone(),
    }
}

/// Optimize loops in a list of statements
fn optimize_loops_in_statements<'ast>(
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
                if let Pattern::Identifier(variable) = pattern {
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
                        let (hoisted, new_body) = hoist_loop_invariants(body, variable, stats, optimizer);
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
                    let final_body = optimize_loops_in_statements(&optimized_body, config, stats, optimizer);

                    result.push(optimizer.alloc_stmt(unsafe { std::mem::transmute(Statement::For {
                        pattern: pattern.clone(),
                        iterable: optimize_loops_in_expression(iterable, config, stats, optimizer),
                        body: final_body,
                        location: location.clone(),
                    }) }));
                } else {
                    // For complex patterns (tuples, etc.), just recursively optimize the body
                    let final_body = optimize_loops_in_statements(body, config, stats, optimizer);
                    result.push(optimizer.alloc_stmt(Statement::For {
                        pattern: pattern.clone(),
                        iterable: optimize_loops_in_expression(iterable, config, stats, optimizer),
                        body: final_body,
                        location: location.clone(),
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
                let final_body = optimize_loops_in_statements(&optimized_body, config, stats, optimizer);

                result.push(optimizer.alloc_stmt(Statement::While {
                    condition: optimize_loops_in_expression(condition, config, stats, optimizer),
                    body: final_body,
                    location: location.clone(),
                }));
            }
            _ => result.push(optimize_loops_in_statement(stmt, config, stats, optimizer)),
        }
    }

    result
}

/// Optimize loops in a single statement
fn optimize_loops_in_statement<'ast>(
    stmt: &'ast Statement<'ast>,
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
    optimizer: &crate::optimizer::Optimizer,
)-> &'ast Statement<'ast> {
    match stmt {
        Statement::Expression { expr, location } => optimizer.alloc_stmt(Statement::Expression {
            expr: optimize_loops_in_expression(expr, config, stats, optimizer),
            location: location.clone(),
        }),
        Statement::Return {
            value: Some(expr),
            location,
        } => optimizer.alloc_stmt(Statement::Return {
            value: Some(optimize_loops_in_expression(expr, config, stats, optimizer)),
            location: location.clone(),
        }),
        Statement::Let {
            pattern,
            mutable,
            type_,
            value,
            else_block,
            location,
        } => optimizer.alloc_stmt(unsafe { std::mem::transmute(Statement::Let {
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
        }) }),
        Statement::Assignment {
            target,
            value,
            compound_op,
            location,
        } => optimizer.alloc_stmt(Statement::Assignment {
            target: target.clone(),
            value: optimize_loops_in_expression(value, config, stats, optimizer),
            compound_op: *compound_op,
            location: location.clone(),
        }),
        Statement::If {
            condition,
            then_block,
            else_block,
            location,
        } => optimizer.alloc_stmt(Statement::If {
            condition: optimize_loops_in_expression(condition, config, stats, optimizer),
            then_block: optimize_loops_in_statements(then_block, config, stats, optimizer),
            else_block: else_block
                .as_ref()
                .map(|stmts| optimize_loops_in_statements(stmts, config, stats, optimizer)),
            location: location.clone(),
        }),
        Statement::Match {
            value,
            arms,
            location,
        } => optimizer.alloc_stmt(Statement::Match {
            value: optimize_loops_in_expression(value, config, stats, optimizer),
            arms: arms
                .iter()
                .map(|arm| crate::parser::MatchArm {
                    pattern: arm.pattern.clone(),
                    guard: arm
                        .guard
                        .as_ref()
                        .map(|g| optimize_loops_in_expression(g, config, stats, optimizer)),
                    body: optimize_loops_in_expression(&arm.body, config, stats, optimizer),
                })
                .collect(),
            location: location.clone(),
        }),
        _ => stmt,
    }
}

/// Optimize loops in an expression
fn optimize_loops_in_expression<'ast>(
    expr: &'ast Expression<'ast>,
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Expression<'ast> {
    match expr {
        Expression::Call {
            function,
            arguments,
            location,
        } => optimizer.alloc_expr(Expression::Call {
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
        }),
        Expression::MethodCall {
            object,
            method,
            type_args,
            arguments,
            location,
        } => optimizer.alloc_expr(Expression::MethodCall {
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
        }),
        Expression::Binary {
            left,
            op,
            right,
            location,
        } => {
            // Apply strength reduction if enabled
            if config.enable_strength_reduction {
                if let Some(reduced) = try_strength_reduction(left, op, right, config, stats, optimizer) {
                    return reduced;
                }
            }

            optimizer.alloc_expr(Expression::Binary {
                left: optimize_loops_in_expression(left, config, stats, optimizer),
                op: *op,
                right: optimize_loops_in_expression(right, config, stats, optimizer),
                location: location.clone(),
            })
        }
        Expression::Unary {
            op,
            operand,
            location,
        } => optimizer.alloc_expr(Expression::Unary {
            op: *op,
            operand: optimize_loops_in_expression(operand, config, stats, optimizer),
            location: location.clone(),
        }),
        Expression::Block {
            statements,
            location,
        } => optimizer.alloc_expr(Expression::Block {
            statements: optimize_loops_in_statements(statements, config, stats, optimizer),
            location: location.clone(),
        }),
        Expression::Closure {
            parameters,
            body,
            location,
        } => optimizer.alloc_expr(Expression::Closure {
            parameters: parameters.clone(),
            body: optimize_loops_in_expression(body, config, stats, optimizer),
            location: location.clone(),
        }),
        Expression::Index {
            object,
            index,
            location,
        } => optimizer.alloc_expr(Expression::Index {
            object: optimize_loops_in_expression(object, config, stats, optimizer),
            index: optimize_loops_in_expression(index, config, stats, optimizer),
            location: location.clone(),
        }),
        Expression::FieldAccess {
            object,
            field,
            location,
        } => optimizer.alloc_expr(Expression::FieldAccess {
            object: optimize_loops_in_expression(object, config, stats, optimizer),
            field: field.clone(),
            location: location.clone(),
        }),
        Expression::Cast {
            expr,
            type_,
            location,
        } => optimizer.alloc_expr(Expression::Cast {
            expr: optimize_loops_in_expression(expr, config, stats, optimizer),
            type_: type_.clone(),
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
                .map(|(k, v)| (k.clone(), optimize_loops_in_expression(v, config, stats, optimizer)))
                .collect(),
            location: location.clone(),
        }),
        Expression::Tuple { elements, location } => optimizer.alloc_expr(Expression::Tuple {
            elements: elements
                .iter()
                .map(|e| optimize_loops_in_expression(e, config, stats, optimizer))
                .collect(),
            location: location.clone(),
        }),
        Expression::Range {
            start,
            end,
            inclusive,
            location,
        } => optimizer.alloc_expr(Expression::Range {
            start: optimize_loops_in_expression(start, config, stats, optimizer),
            end: optimize_loops_in_expression(end, config, stats, optimizer),
            inclusive: *inclusive,
            location: location.clone(),
        }),
        Expression::ChannelSend {
            channel,
            value,
            location,
        } => optimizer.alloc_expr(Expression::ChannelSend {
            channel: optimize_loops_in_expression(channel, config, stats, optimizer),
            value: optimize_loops_in_expression(value, config, stats, optimizer),
            location: location.clone(),
        }),
        Expression::ChannelRecv { channel, location } => optimizer.alloc_expr(Expression::ChannelRecv {
            channel: optimize_loops_in_expression(channel, config, stats, optimizer),
            location: location.clone(),
        }),
        Expression::Await { expr, location } => optimizer.alloc_expr(Expression::Await {
            expr: optimize_loops_in_expression(expr, config, stats, optimizer),
            location: location.clone(),
        }),
        Expression::TryOp { expr, location } => optimizer.alloc_expr(unsafe { std::mem::transmute(Expression::TryOp {
            expr: optimize_loops_in_expression(expr, config, stats, optimizer),
            location: location.clone(),
        }) }),
        Expression::MacroInvocation {
            name,
            args,
            delimiter,
            location,
        } => optimizer.alloc_expr(Expression::MacroInvocation {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| optimize_loops_in_expression(a, config, stats, optimizer))
                .collect(),
            delimiter: *delimiter,
            location: location.clone(),
        }),
        _ => expr,
    }
}

/// Try to unroll a loop if it's a small constant range
fn try_unroll_loop<'ast>(
    variable: &str,
    iterable: &'ast Expression<'ast>,
    body: &[&'ast Statement<'ast>],
    config: &LoopOptimizationConfig,
    _stats: &mut LoopOptimizationStats,
    optimizer: &crate::optimizer::Optimizer,
) -> Option<Vec<&'ast Statement<'ast>>> {
    // Only unroll simple range expressions: 0..n or 0..=n
    if let Expression::Range {
        start,
        end,
        inclusive,
        ..
    } = iterable
    {
        // Check if start is 0
        if let Expression::Literal {
            value: crate::parser::Literal::Int(start_val),
            ..
        } = &**start
        {
            if *start_val != 0 {
                return None;
            }

            // Check if end is a constant
            if let Expression::Literal {
                value: crate::parser::Literal::Int(end_val),
                ..
            } = &**end
            {
                let iterations = if *inclusive {
                    (*end_val + 1) as usize
                } else {
                    *end_val as usize
                };

                // Only unroll if within configured limit
                if iterations > config.max_unroll_iterations || iterations == 0 {
                    return None;
                }

                // Unroll the loop
                let mut unrolled = Vec::new();
                for i in 0..iterations {
                    // Replace loop variable with the current iteration value
                    let iter_expr = Expression::Literal {
                        value: crate::parser::Literal::Int(i as i64),
                        location: None,
                    };
                    for stmt in body {
                        unrolled.push(replace_variable_in_statement(stmt, variable, &iter_expr, optimizer));
                    }
                }

                return Some(unrolled);
            }
        }
    }

    None
}

/// Hoist loop-invariant statements outside the loop
fn hoist_loop_invariants<'ast>(
    body: &[&'ast Statement<'ast>],
    loop_var: &str,
    stats: &mut LoopOptimizationStats,
    optimizer: &crate::optimizer::Optimizer,
) -> (Vec<&'ast Statement<'ast>>, Vec<&'ast Statement<'ast>>) {
    let mut hoisted = Vec::new();
    let mut remaining = Vec::new();

    for stmt in body {
        if is_loop_invariant(stmt, loop_var) {
            hoisted.push(stmt.clone());
            stats.invariants_hoisted += 1;
        } else {
            remaining.push(stmt.clone());
        }
    }

    (hoisted, remaining)
}

/// Check if a statement is loop-invariant (doesn't depend on loop variable)
fn is_loop_invariant<'ast>(stmt: &'ast Statement<'ast>, loop_var: &str) -> bool {
    // Only hoist Let statements that don't depend on the loop variable
    match stmt {
        Statement::Let { value, .. } => !expression_uses_variable(value, loop_var),
        _ => false,
    }
}

/// Check if an expression uses a specific variable
fn expression_uses_variable<'ast>(expr: &'ast Expression<'ast>, var_name: &str) -> bool {
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
fn statement_uses_variable<'ast>(stmt: &'ast Statement<'ast>, var_name: &str) -> bool {
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
                        || expression_uses_variable(&arm.body, var_name)
                })
        }
        _ => false,
    }
}

/// Replace all occurrences of a variable in a statement with an expression
fn replace_variable_in_statement<'a, 'ast>(
    stmt: &'a Statement<'a>,
    var_name: &str,
    replacement: &'a Expression<'a>,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Statement<'ast> {
    match stmt {
        Statement::Expression { expr, .. } => optimizer.alloc_stmt(Statement::Expression {
            expr: replace_variable_in_expression(expr, var_name, replacement, optimizer),
            location: None,
        }),
        Statement::Return {
            value: Some(expr), ..
        } => optimizer.alloc_stmt(Statement::Return {
            value: Some(replace_variable_in_expression(expr, var_name, replacement, optimizer)),
            location: None,
        }),
        Statement::Let {
            pattern,
            mutable,
            type_,
            value,
            else_block,
            ..
        } => optimizer.alloc_stmt(unsafe { std::mem::transmute(Statement::Let {
            pattern: pattern.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: replace_variable_in_expression(value, var_name, replacement, optimizer),
            else_block: else_block.clone(),
            location: None,
        }) }),
        Statement::Assignment { target, value, .. } => optimizer.alloc_stmt(Statement::Assignment {
            target: replace_variable_in_expression(target, var_name, replacement, optimizer),
            value: replace_variable_in_expression(value, var_name, replacement, optimizer),
            compound_op: None,
            location: None,
        }),
        _ => unsafe { std::mem::transmute(stmt) }, // Safe: just changing lifetime annotation
    }
}

/// Replace all occurrences of a variable in an expression with another expression
fn replace_variable_in_expression<'a, 'ast>(
    expr: &'a Expression<'a>,
    var_name: &str,
    replacement: &'a Expression<'a>,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Expression<'ast> {
    match expr {
        Expression::Identifier { name, .. } if name == var_name => unsafe { std::mem::transmute(replacement) },
        Expression::Binary {
            left, op, right, ..
        } => optimizer.alloc_expr(Expression::Binary {
            left: replace_variable_in_expression(left, var_name, replacement, optimizer),
            op: *op,
            right: replace_variable_in_expression(right, var_name, replacement, optimizer),
            location: None,
        }),
        Expression::Unary { op, operand, .. } => optimizer.alloc_expr(Expression::Unary {
            op: *op,
            operand: replace_variable_in_expression(
                operand,
                var_name,
                replacement,
                optimizer,
            ),
            location: None,
        }),
        Expression::Index { object, index, .. } => optimizer.alloc_expr(Expression::Index {
            object: replace_variable_in_expression(
                object,
                var_name,
                replacement,
                optimizer,
            ),
            index: replace_variable_in_expression(index, var_name, replacement, optimizer),
            location: None,
        }),
        _ => unsafe { std::mem::transmute(expr) }, // Safe: just changing lifetime annotation
    }
}

/// Try to apply strength reduction to binary operations
fn try_strength_reduction<'ast>(
    _left: &'ast Expression<'ast>,
    _op: &BinaryOp,
    _right: &'ast Expression<'ast>,
    _config: &LoopOptimizationConfig,
    _stats: &mut LoopOptimizationStats,
    _optimizer: &crate::optimizer::Optimizer,
) -> Option<&'ast Expression<'ast>> {
    // Note: Strength reduction like x * 2 -> x << 1 would require additional operators in BinaryOp
    // For now, we return None but this is a placeholder for future optimizations
    // such as replacing expensive operations with cheaper ones (e.g., x * 1 -> x, x * 0 -> 0)
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Decorator, Literal, Type};

    #[test]
    fn test_loop_unrolling_simple() {
        let program = Program {
            items: vec![Item::Function {
                decl: FunctionDecl {
                    is_pub: false,
                    is_extern: false,
                    name: "test".to_string(),
                    type_params: vec![],
                    where_clause: vec![],
                    decorators: vec![Decorator {
                        name: "pub".to_string(),
                        arguments: vec![],
                    }],
                    is_async: false,
                    parent_type: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: None,
                    body: vec![Statement::For {
                        pattern: Pattern::Identifier("i".to_string()),
                        iterable: Expression::Range {
                            start: Box::new(Expression::Literal {
                                value: Literal::Int(0),
                                location: None,
                            }),
                            end: Box::new(Expression::Literal {
                                value: Literal::Int(3),
                                location: None,
                            }),
                            inclusive: false,
                            location: None,
                        },
                        body: vec![Statement::Expression {
                            expr: Expression::MacroInvocation {
                                name: "println".to_string(),
                                args: vec![Expression::Identifier {
                                    name: "i".to_string(),
                                    location: None,
                                }],
                                delimiter: crate::parser::MacroDelimiter::Parens,
                                location: None,
                            },
                            location: None,
                        }],
                        location: None,
                    }],
                },
                location: None,
            }],
        };

        let (optimized, stats) = optimize_loops(&program);
        assert_eq!(stats.loops_unrolled, 1);

        let func = match &optimized.items[0] {
            Item::Function { decl: f, .. } => f,
            _ => panic!("Expected function"),
        };
        // After unrolling, we should have 3 statements instead of 1 loop
        assert_eq!(func.body.len(), 3);
    }

    #[test]
    fn test_licm_hoisting() {
        let program = Program {
            items: vec![Item::Function {
                decl: FunctionDecl {
                    is_pub: false,
                    is_extern: false,
                    name: "test".to_string(),
                    type_params: vec![],
                    where_clause: vec![],
                    decorators: vec![],
                    is_async: false,
                    parent_type: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: None,
                    body: vec![Statement::For {
                        pattern: Pattern::Identifier("i".to_string()),
                        iterable: Expression::Range {
                            start: Box::new(Expression::Literal {
                                value: Literal::Int(0),
                                location: None,
                            }),
                            end: Box::new(Expression::Literal {
                                value: Literal::Int(100),
                                location: None,
                            }),
                            inclusive: false,
                            location: None,
                        },
                        body: vec![
                            // Loop-invariant: doesn't use 'i'
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
                            // Loop-variant: uses 'i'
                            Statement::Expression {
                                expr: Expression::Binary {
                                    left: Box::new(Expression::Identifier {
                                        name: "x".to_string(),
                                        location: None,
                                    }),
                                    op: BinaryOp::Add,
                                    right: Box::new(Expression::Identifier {
                                        name: "i".to_string(),
                                        location: None,
                                    }),
                                    location: None,
                                },
                                location: None,
                            },
                        ],
                        location: None,
                    }],
                },
                location: None,
            }],
        };

        let (optimized, stats) = optimize_loops(&program);
        assert_eq!(stats.invariants_hoisted, 1);
        assert_eq!(stats.loops_optimized, 1);

        let func = match &optimized.items[0] {
            Item::Function { decl: f, .. } => f,
            _ => panic!("Expected function"),
        };
        // The hoisted statement should come before the loop
        assert_eq!(func.body.len(), 2); // hoisted let + for loop
    }

    #[test]
    fn test_strength_reduction_placeholder() {
        // Placeholder test for strength reduction
        // Currently no strength reductions are implemented due to limited BinaryOp variants
        let program = Program {
            items: vec![Item::Function {
                decl: FunctionDecl {
                    is_pub: false,
                    is_extern: false,
                    name: "test".to_string(),
                    type_params: vec![],
                    where_clause: vec![],
                    decorators: vec![],
                    is_async: false,
                    parent_type: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: Some(Type::Custom("i32".to_string())),
                    body: vec![Statement::Return {
                        value: Some(Expression::Binary {
                            left: Box::new(Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            }),
                            op: BinaryOp::Mul,
                            right: Box::new(Expression::Literal {
                                value: Literal::Int(4),
                                location: None,
                            }),
                            location: None,
                        }),
                        location: None,
                    }],
                },
                location: None,
            }],
        };

        let (_, stats) = optimize_loops(&program);
        // No strength reductions implemented yet
        assert_eq!(stats.strength_reductions, 0);
    }

    #[test]
    fn test_no_unrolling_for_large_loops() {
        let program = Program {
            items: vec![Item::Function {
                decl: FunctionDecl {
                    is_pub: false,
                    is_extern: false,
                    name: "test".to_string(),
                    type_params: vec![],
                    where_clause: vec![],
                    decorators: vec![],
                    is_async: false,
                    parent_type: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: None,
                    body: vec![Statement::For {
                        pattern: Pattern::Identifier("i".to_string()),
                        iterable: Expression::Range {
                            start: Box::new(Expression::Literal {
                                value: Literal::Int(0),
                                location: None,
                            }),
                            end: Box::new(Expression::Literal {
                                value: Literal::Int(1000),
                                location: None,
                            }), // Too large
                            inclusive: false,
                            location: None,
                        },
                        body: vec![Statement::Expression {
                            expr: Expression::Identifier {
                                name: "i".to_string(),
                                location: None,
                            },
                            location: None,
                        }],
                        location: None,
                    }],
                },
                location: None,
            }],
        };

        let (optimized, stats) = optimize_loops(&program);
        assert_eq!(stats.loops_unrolled, 0); // Should not unroll

        let func = match &optimized.items[0] {
            Item::Function { decl: f, .. } => f,
            _ => panic!("Expected function"),
        };
        // Loop should remain
        assert_eq!(func.body.len(), 1);
        assert!(matches!(func.body[0], Statement::For { .. }));
    }

    #[test]
    fn test_no_hoisting_for_variant_code() {
        let program = Program {
            items: vec![Item::Function {
                decl: FunctionDecl {
                    is_pub: false,
                    is_extern: false,
                    name: "test".to_string(),
                    type_params: vec![],
                    where_clause: vec![],
                    decorators: vec![],
                    is_async: false,
                    parent_type: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: None,
                    body: vec![Statement::For {
                        pattern: Pattern::Identifier("i".to_string()),
                        iterable: Expression::Range {
                            start: Box::new(Expression::Literal {
                                value: Literal::Int(0),
                                location: None,
                            }),
                            end: Box::new(Expression::Literal {
                                value: Literal::Int(10),
                                location: None,
                            }),
                            inclusive: false,
                            location: None,
                        },
                        body: vec![
                            // Loop-variant: uses 'i'
                            Statement::Let {
                                pattern: Pattern::Identifier("x".to_string()),
                                mutable: false,
                                type_: None,
                                value: Expression::Binary {
                                    left: Box::new(Expression::Identifier {
                                        name: "i".to_string(),
                                        location: None,
                                    }),
                                    op: BinaryOp::Mul,
                                    right: Box::new(Expression::Literal {
                                        value: Literal::Int(2),
                                        location: None,
                                    }),
                                    location: None,
                                },
                                else_block: None,
                                location: None,
                            },
                        ],
                        location: None,
                    }],
                },
                location: None,
            }],
        };

        let (_, stats) = optimize_loops(&program);
        assert_eq!(stats.invariants_hoisted, 0); // Should not hoist
    }
}
