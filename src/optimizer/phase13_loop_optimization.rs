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

use crate::parser::{BinaryOp, Expression, FunctionDecl, ImplBlock, Item, Program, Statement};
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
pub fn optimize_loops(program: &Program) -> (Program, LoopOptimizationStats) {
    optimize_loops_with_config(program, &LoopOptimizationConfig::default())
}

/// Optimize loops with custom configuration
pub fn optimize_loops_with_config(
    program: &Program,
    config: &LoopOptimizationConfig,
) -> (Program, LoopOptimizationStats) {
    let mut stats = LoopOptimizationStats::default();

    let new_items = program
        .items
        .iter()
        .map(|item| optimize_loops_in_item(item, config, &mut stats))
        .collect();

    (Program { items: new_items }, stats)
}

/// Optimize loops in a single item
fn optimize_loops_in_item(
    item: &Item,
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
) -> Item {
    match item {
        Item::Function(func) => {
            let new_body = optimize_loops_in_statements(&func.body, config, stats);
            Item::Function(FunctionDecl {
                name: func.name.clone(),
                type_params: func.type_params.clone(),
                where_clause: func.where_clause.clone(),
                decorators: func.decorators.clone(),
                is_async: func.is_async,
                parameters: func.parameters.clone(),
                return_type: func.return_type.clone(),
                body: new_body,
            })
        }
        Item::Impl(impl_block) => {
            let new_functions = impl_block
                .functions
                .iter()
                .map(|func| FunctionDecl {
                    name: func.name.clone(),
                    type_params: func.type_params.clone(),
                    where_clause: func.where_clause.clone(),
                    decorators: func.decorators.clone(),
                    is_async: func.is_async,
                    parameters: func.parameters.clone(),
                    return_type: func.return_type.clone(),
                    body: optimize_loops_in_statements(&func.body, config, stats),
                })
                .collect();

            Item::Impl(ImplBlock {
                type_name: impl_block.type_name.clone(),
                type_params: impl_block.type_params.clone(),
                where_clause: impl_block.where_clause.clone(),
                trait_name: impl_block.trait_name.clone(),
                trait_type_args: impl_block.trait_type_args.clone(),
                associated_types: impl_block.associated_types.clone(),
                functions: new_functions,
                decorators: impl_block.decorators.clone(),
            })
        }
        Item::Static {
            name,
            mutable,
            type_,
            value,
        } => Item::Static {
            name: name.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: optimize_loops_in_expression(value, config, stats),
        },
        Item::Const { name, type_, value } => Item::Const {
            name: name.clone(),
            type_: type_.clone(),
            value: optimize_loops_in_expression(value, config, stats),
        },
        _ => item.clone(),
    }
}

/// Optimize loops in a list of statements
fn optimize_loops_in_statements(
    statements: &[Statement],
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
) -> Vec<Statement> {
    let mut result = Vec::new();

    for stmt in statements {
        match stmt {
            Statement::For {
                variable,
                iterable,
                body,
            } => {
                // Try to unroll the loop if it's a small constant range
                if config.enable_unrolling {
                    if let Some(unrolled) = try_unroll_loop(variable, iterable, body, config, stats)
                    {
                        result.extend(unrolled);
                        stats.loops_unrolled += 1;
                        stats.loops_optimized += 1;
                        continue;
                    }
                }

                // Apply LICM if enabled
                let optimized_body = if config.enable_licm {
                    let (hoisted, new_body) = hoist_loop_invariants(body, variable, stats);
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
                let final_body = optimize_loops_in_statements(&optimized_body, config, stats);

                result.push(Statement::For {
                    variable: variable.clone(),
                    iterable: optimize_loops_in_expression(iterable, config, stats),
                    body: final_body,
                });
            }
            Statement::While { condition, body } => {
                // Apply LICM if enabled
                let optimized_body = if config.enable_licm {
                    let (hoisted, new_body) = hoist_loop_invariants(body, "", stats);
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
                let final_body = optimize_loops_in_statements(&optimized_body, config, stats);

                result.push(Statement::While {
                    condition: optimize_loops_in_expression(condition, config, stats),
                    body: final_body,
                });
            }
            _ => result.push(optimize_loops_in_statement(stmt, config, stats)),
        }
    }

    result
}

/// Optimize loops in a single statement
fn optimize_loops_in_statement(
    stmt: &Statement,
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
) -> Statement {
    match stmt {
        Statement::Expression(expr) => {
            Statement::Expression(optimize_loops_in_expression(expr, config, stats))
        }
        Statement::Return(Some(expr)) => {
            Statement::Return(Some(optimize_loops_in_expression(expr, config, stats)))
        }
        Statement::Let {
            name,
            mutable,
            type_,
            value,
        } => Statement::Let {
            name: name.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: optimize_loops_in_expression(value, config, stats),
        },
        Statement::Assignment { target, value } => Statement::Assignment {
            target: target.clone(),
            value: optimize_loops_in_expression(value, config, stats),
        },
        Statement::If {
            condition,
            then_block,
            else_block,
        } => Statement::If {
            condition: optimize_loops_in_expression(condition, config, stats),
            then_block: optimize_loops_in_statements(then_block, config, stats),
            else_block: else_block
                .as_ref()
                .map(|stmts| optimize_loops_in_statements(stmts, config, stats)),
        },
        Statement::Match { value, arms } => Statement::Match {
            value: optimize_loops_in_expression(value, config, stats),
            arms: arms
                .iter()
                .map(|arm| crate::parser::MatchArm {
                    pattern: arm.pattern.clone(),
                    guard: arm
                        .guard
                        .as_ref()
                        .map(|g| optimize_loops_in_expression(g, config, stats)),
                    body: optimize_loops_in_expression(&arm.body, config, stats),
                })
                .collect(),
        },
        _ => stmt.clone(),
    }
}

/// Optimize loops in an expression
fn optimize_loops_in_expression(
    expr: &Expression,
    config: &LoopOptimizationConfig,
    stats: &mut LoopOptimizationStats,
) -> Expression {
    match expr {
        Expression::Call {
            function,
            arguments,
        } => Expression::Call {
            function: Box::new(optimize_loops_in_expression(function, config, stats)),
            arguments: arguments
                .iter()
                .map(|(label, arg)| {
                    (
                        label.clone(),
                        optimize_loops_in_expression(arg, config, stats),
                    )
                })
                .collect(),
        },
        Expression::MethodCall {
            object,
            method,
            type_args,
            arguments,
        } => Expression::MethodCall {
            object: Box::new(optimize_loops_in_expression(object, config, stats)),
            method: method.clone(),
            type_args: type_args.clone(),
            arguments: arguments
                .iter()
                .map(|(label, arg)| {
                    (
                        label.clone(),
                        optimize_loops_in_expression(arg, config, stats),
                    )
                })
                .collect(),
        },
        Expression::Binary { left, op, right } => {
            // Apply strength reduction if enabled
            if config.enable_strength_reduction {
                if let Some(reduced) = try_strength_reduction(left, op, right, config, stats) {
                    return reduced;
                }
            }

            Expression::Binary {
                left: Box::new(optimize_loops_in_expression(left, config, stats)),
                op: op.clone(),
                right: Box::new(optimize_loops_in_expression(right, config, stats)),
            }
        }
        Expression::Unary { op, operand } => Expression::Unary {
            op: op.clone(),
            operand: Box::new(optimize_loops_in_expression(operand, config, stats)),
        },
        Expression::Block(statements) => {
            Expression::Block(optimize_loops_in_statements(statements, config, stats))
        }
        Expression::Closure { parameters, body } => Expression::Closure {
            parameters: parameters.clone(),
            body: Box::new(optimize_loops_in_expression(body, config, stats)),
        },
        Expression::Ternary {
            condition,
            true_expr,
            false_expr,
        } => Expression::Ternary {
            condition: Box::new(optimize_loops_in_expression(condition, config, stats)),
            true_expr: Box::new(optimize_loops_in_expression(true_expr, config, stats)),
            false_expr: Box::new(optimize_loops_in_expression(false_expr, config, stats)),
        },
        Expression::Index { object, index } => Expression::Index {
            object: Box::new(optimize_loops_in_expression(object, config, stats)),
            index: Box::new(optimize_loops_in_expression(index, config, stats)),
        },
        Expression::FieldAccess { object, field } => Expression::FieldAccess {
            object: Box::new(optimize_loops_in_expression(object, config, stats)),
            field: field.clone(),
        },
        Expression::Cast { expr, type_ } => Expression::Cast {
            expr: Box::new(optimize_loops_in_expression(expr, config, stats)),
            type_: type_.clone(),
        },
        Expression::StructLiteral { name, fields } => Expression::StructLiteral {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), optimize_loops_in_expression(v, config, stats)))
                .collect(),
        },
        Expression::Tuple(elements) => Expression::Tuple(
            elements
                .iter()
                .map(|e| optimize_loops_in_expression(e, config, stats))
                .collect(),
        ),
        Expression::Range {
            start,
            end,
            inclusive,
        } => Expression::Range {
            start: Box::new(optimize_loops_in_expression(start, config, stats)),
            end: Box::new(optimize_loops_in_expression(end, config, stats)),
            inclusive: *inclusive,
        },
        Expression::ChannelSend { channel, value } => Expression::ChannelSend {
            channel: Box::new(optimize_loops_in_expression(channel, config, stats)),
            value: Box::new(optimize_loops_in_expression(value, config, stats)),
        },
        Expression::ChannelRecv(channel) => Expression::ChannelRecv(Box::new(
            optimize_loops_in_expression(channel, config, stats),
        )),
        Expression::Await(expr) => {
            Expression::Await(Box::new(optimize_loops_in_expression(expr, config, stats)))
        }
        Expression::TryOp(expr) => {
            Expression::TryOp(Box::new(optimize_loops_in_expression(expr, config, stats)))
        }
        Expression::MacroInvocation {
            name,
            args,
            delimiter,
        } => Expression::MacroInvocation {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| optimize_loops_in_expression(a, config, stats))
                .collect(),
            delimiter: delimiter.clone(),
        },
        _ => expr.clone(),
    }
}

/// Try to unroll a loop if it's a small constant range
fn try_unroll_loop(
    variable: &str,
    iterable: &Expression,
    body: &[Statement],
    config: &LoopOptimizationConfig,
    _stats: &mut LoopOptimizationStats,
) -> Option<Vec<Statement>> {
    // Only unroll simple range expressions: 0..n or 0..=n
    if let Expression::Range {
        start,
        end,
        inclusive,
    } = iterable
    {
        // Check if start is 0
        if let Expression::Literal(crate::parser::Literal::Int(start_val)) = &**start {
            if *start_val != 0 {
                return None;
            }

            // Check if end is a constant
            if let Expression::Literal(crate::parser::Literal::Int(end_val)) = &**end {
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
                    let iter_expr = Expression::Literal(crate::parser::Literal::Int(i as i64));
                    for stmt in body {
                        unrolled.push(replace_variable_in_statement(stmt, variable, &iter_expr));
                    }
                }

                return Some(unrolled);
            }
        }
    }

    None
}

/// Hoist loop-invariant statements outside the loop
fn hoist_loop_invariants(
    body: &[Statement],
    loop_var: &str,
    stats: &mut LoopOptimizationStats,
) -> (Vec<Statement>, Vec<Statement>) {
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
fn is_loop_invariant(stmt: &Statement, loop_var: &str) -> bool {
    // Only hoist Let statements that don't depend on the loop variable
    match stmt {
        Statement::Let { value, .. } => !expression_uses_variable(value, loop_var),
        _ => false,
    }
}

/// Check if an expression uses a specific variable
fn expression_uses_variable(expr: &Expression, var_name: &str) -> bool {
    match expr {
        Expression::Identifier(name) => name == var_name,
        Expression::Binary { left, right, .. } => {
            expression_uses_variable(left, var_name) || expression_uses_variable(right, var_name)
        }
        Expression::Unary { operand, .. } => expression_uses_variable(operand, var_name),
        Expression::Call {
            function,
            arguments,
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
        Expression::Index { object, index } => {
            expression_uses_variable(object, var_name) || expression_uses_variable(index, var_name)
        }
        Expression::FieldAccess { object, .. } => expression_uses_variable(object, var_name),
        Expression::Cast { expr, .. } => expression_uses_variable(expr, var_name),
        Expression::Tuple(elements) => elements
            .iter()
            .any(|e| expression_uses_variable(e, var_name)),
        Expression::StructLiteral { fields, .. } => fields
            .iter()
            .any(|(_, v)| expression_uses_variable(v, var_name)),
        Expression::Range { start, end, .. } => {
            expression_uses_variable(start, var_name) || expression_uses_variable(end, var_name)
        }
        Expression::Ternary {
            condition,
            true_expr,
            false_expr,
        } => {
            expression_uses_variable(condition, var_name)
                || expression_uses_variable(true_expr, var_name)
                || expression_uses_variable(false_expr, var_name)
        }
        Expression::Closure { body, .. } => expression_uses_variable(body, var_name),
        Expression::Block(statements) => statements
            .iter()
            .any(|s| statement_uses_variable(s, var_name)),
        Expression::ChannelSend { channel, value } => {
            expression_uses_variable(channel, var_name) || expression_uses_variable(value, var_name)
        }
        Expression::ChannelRecv(channel) => expression_uses_variable(channel, var_name),
        Expression::Await(expr) | Expression::TryOp(expr) => {
            expression_uses_variable(expr, var_name)
        }
        Expression::MacroInvocation { args, .. } => {
            args.iter().any(|a| expression_uses_variable(a, var_name))
        }
        _ => false,
    }
}

/// Check if a statement uses a specific variable
fn statement_uses_variable(stmt: &Statement, var_name: &str) -> bool {
    match stmt {
        Statement::Expression(expr) | Statement::Return(Some(expr)) => {
            expression_uses_variable(expr, var_name)
        }
        Statement::Let { value, .. } => expression_uses_variable(value, var_name),
        Statement::Assignment { target, value } => {
            expression_uses_variable(target, var_name) || expression_uses_variable(value, var_name)
        }
        Statement::If {
            condition,
            then_block,
            else_block,
        } => {
            expression_uses_variable(condition, var_name)
                || then_block
                    .iter()
                    .any(|s| statement_uses_variable(s, var_name))
                || else_block.as_ref().map_or(false, |stmts| {
                    stmts.iter().any(|s| statement_uses_variable(s, var_name))
                })
        }
        Statement::While { condition, body } => {
            expression_uses_variable(condition, var_name)
                || body.iter().any(|s| statement_uses_variable(s, var_name))
        }
        Statement::For {
            variable,
            iterable,
            body,
        } => {
            // If this is a nested loop with the same variable, it shadows the outer one
            if variable == var_name {
                return false;
            }
            expression_uses_variable(iterable, var_name)
                || body.iter().any(|s| statement_uses_variable(s, var_name))
        }
        Statement::Match { value, arms } => {
            expression_uses_variable(value, var_name)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .map_or(false, |g| expression_uses_variable(g, var_name))
                        || expression_uses_variable(&arm.body, var_name)
                })
        }
        _ => false,
    }
}

/// Replace all occurrences of a variable in a statement with an expression
fn replace_variable_in_statement(
    stmt: &Statement,
    var_name: &str,
    replacement: &Expression,
) -> Statement {
    match stmt {
        Statement::Expression(expr) => {
            Statement::Expression(replace_variable_in_expression(expr, var_name, replacement))
        }
        Statement::Return(Some(expr)) => Statement::Return(Some(replace_variable_in_expression(
            expr,
            var_name,
            replacement,
        ))),
        Statement::Let {
            name,
            mutable,
            type_,
            value,
        } => Statement::Let {
            name: name.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: replace_variable_in_expression(value, var_name, replacement),
        },
        Statement::Assignment { target, value } => Statement::Assignment {
            target: replace_variable_in_expression(target, var_name, replacement),
            value: replace_variable_in_expression(value, var_name, replacement),
        },
        _ => stmt.clone(),
    }
}

/// Replace all occurrences of a variable in an expression with another expression
fn replace_variable_in_expression(
    expr: &Expression,
    var_name: &str,
    replacement: &Expression,
) -> Expression {
    match expr {
        Expression::Identifier(name) if name == var_name => replacement.clone(),
        Expression::Binary { left, op, right } => Expression::Binary {
            left: Box::new(replace_variable_in_expression(left, var_name, replacement)),
            op: op.clone(),
            right: Box::new(replace_variable_in_expression(right, var_name, replacement)),
        },
        Expression::Unary { op, operand } => Expression::Unary {
            op: op.clone(),
            operand: Box::new(replace_variable_in_expression(
                operand,
                var_name,
                replacement,
            )),
        },
        Expression::Index { object, index } => Expression::Index {
            object: Box::new(replace_variable_in_expression(
                object,
                var_name,
                replacement,
            )),
            index: Box::new(replace_variable_in_expression(index, var_name, replacement)),
        },
        _ => expr.clone(),
    }
}

/// Try to apply strength reduction to binary operations
fn try_strength_reduction(
    _left: &Expression,
    _op: &BinaryOp,
    _right: &Expression,
    _config: &LoopOptimizationConfig,
    _stats: &mut LoopOptimizationStats,
) -> Option<Expression> {
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
            items: vec![Item::Function(FunctionDecl {
                name: "test".to_string(),
                type_params: vec![],
                where_clause: vec![],
                decorators: vec![Decorator {
                    name: "pub".to_string(),
                    arguments: vec![],
                }],
                is_async: false,
                parameters: vec![],
                return_type: None,
                body: vec![Statement::For {
                    variable: "i".to_string(),
                    iterable: Expression::Range {
                        start: Box::new(Expression::Literal(Literal::Int(0))),
                        end: Box::new(Expression::Literal(Literal::Int(3))),
                        inclusive: false,
                    },
                    body: vec![Statement::Expression(Expression::MacroInvocation {
                        name: "println".to_string(),
                        args: vec![Expression::Identifier("i".to_string())],
                        delimiter: crate::parser::MacroDelimiter::Parens,
                    })],
                }],
            })],
        };

        let (optimized, stats) = optimize_loops(&program);
        assert_eq!(stats.loops_unrolled, 1);

        let func = match &optimized.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };
        // After unrolling, we should have 3 statements instead of 1 loop
        assert_eq!(func.body.len(), 3);
    }

    #[test]
    fn test_licm_hoisting() {
        let program = Program {
            items: vec![Item::Function(FunctionDecl {
                name: "test".to_string(),
                type_params: vec![],
                where_clause: vec![],
                decorators: vec![],
                is_async: false,
                parameters: vec![],
                return_type: None,
                body: vec![Statement::For {
                    variable: "i".to_string(),
                    iterable: Expression::Range {
                        start: Box::new(Expression::Literal(Literal::Int(0))),
                        end: Box::new(Expression::Literal(Literal::Int(100))),
                        inclusive: false,
                    },
                    body: vec![
                        // Loop-invariant: doesn't use 'i'
                        Statement::Let {
                            name: "x".to_string(),
                            mutable: false,
                            type_: None,
                            value: Expression::Literal(Literal::Int(42)),
                        },
                        // Loop-variant: uses 'i'
                        Statement::Expression(Expression::Binary {
                            left: Box::new(Expression::Identifier("x".to_string())),
                            op: BinaryOp::Add,
                            right: Box::new(Expression::Identifier("i".to_string())),
                        }),
                    ],
                }],
            })],
        };

        let (optimized, stats) = optimize_loops(&program);
        assert_eq!(stats.invariants_hoisted, 1);
        assert_eq!(stats.loops_optimized, 1);

        let func = match &optimized.items[0] {
            Item::Function(f) => f,
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
            items: vec![Item::Function(FunctionDecl {
                name: "test".to_string(),
                type_params: vec![],
                where_clause: vec![],
                decorators: vec![],
                is_async: false,
                parameters: vec![],
                return_type: Some(Type::Custom("i32".to_string())),
                body: vec![Statement::Return(Some(Expression::Binary {
                    left: Box::new(Expression::Identifier("x".to_string())),
                    op: BinaryOp::Mul,
                    right: Box::new(Expression::Literal(Literal::Int(4))),
                }))],
            })],
        };

        let (_, stats) = optimize_loops(&program);
        // No strength reductions implemented yet
        assert_eq!(stats.strength_reductions, 0);
    }

    #[test]
    fn test_no_unrolling_for_large_loops() {
        let program = Program {
            items: vec![Item::Function(FunctionDecl {
                name: "test".to_string(),
                type_params: vec![],
                where_clause: vec![],
                decorators: vec![],
                is_async: false,
                parameters: vec![],
                return_type: None,
                body: vec![Statement::For {
                    variable: "i".to_string(),
                    iterable: Expression::Range {
                        start: Box::new(Expression::Literal(Literal::Int(0))),
                        end: Box::new(Expression::Literal(Literal::Int(1000))), // Too large
                        inclusive: false,
                    },
                    body: vec![Statement::Expression(Expression::Identifier(
                        "i".to_string(),
                    ))],
                }],
            })],
        };

        let (optimized, stats) = optimize_loops(&program);
        assert_eq!(stats.loops_unrolled, 0); // Should not unroll

        let func = match &optimized.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };
        // Loop should remain
        assert_eq!(func.body.len(), 1);
        assert!(matches!(func.body[0], Statement::For { .. }));
    }

    #[test]
    fn test_no_hoisting_for_variant_code() {
        let program = Program {
            items: vec![Item::Function(FunctionDecl {
                name: "test".to_string(),
                type_params: vec![],
                where_clause: vec![],
                decorators: vec![],
                is_async: false,
                parameters: vec![],
                return_type: None,
                body: vec![Statement::For {
                    variable: "i".to_string(),
                    iterable: Expression::Range {
                        start: Box::new(Expression::Literal(Literal::Int(0))),
                        end: Box::new(Expression::Literal(Literal::Int(10))),
                        inclusive: false,
                    },
                    body: vec![
                        // Loop-variant: uses 'i'
                        Statement::Let {
                            name: "x".to_string(),
                            mutable: false,
                            type_: None,
                            value: Expression::Binary {
                                left: Box::new(Expression::Identifier("i".to_string())),
                                op: BinaryOp::Mul,
                                right: Box::new(Expression::Literal(Literal::Int(2))),
                            },
                        },
                    ],
                }],
            })],
        };

        let (_, stats) = optimize_loops(&program);
        assert_eq!(stats.invariants_hoisted, 0); // Should not hoist
    }
}
