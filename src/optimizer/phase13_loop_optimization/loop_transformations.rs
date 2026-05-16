//! Loop optimization transforms: unrolling, variable substitution, strength reduction.

use crate::parser::{BinaryOp, Expression, Statement};

use super::{LoopOptimizationConfig, LoopOptimizationStats};

/// Try to unroll a loop if it's a small constant range
pub(in crate::optimizer) fn try_unroll_loop<'ast>(
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
                        unrolled.push(replace_variable_in_statement(
                            stmt, variable, &iter_expr, optimizer,
                        ));
                    }
                }

                return Some(unrolled);
            }
        }
    }

    None
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
            value: Some(replace_variable_in_expression(
                expr,
                var_name,
                replacement,
                optimizer,
            )),
            location: None,
        }),
        Statement::Let {
            pattern,
            mutable,
            type_,
            value,
            else_block,
            ..
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Let {
                pattern: pattern.clone(),
                mutable: *mutable,
                type_: type_.clone(),
                value: replace_variable_in_expression(value, var_name, replacement, optimizer),
                else_block: else_block.clone(),
                location: None,
            })
        }),
        Statement::Assignment { target, value, .. } => {
            optimizer.alloc_stmt(Statement::Assignment {
                target: replace_variable_in_expression(target, var_name, replacement, optimizer),
                value: replace_variable_in_expression(value, var_name, replacement, optimizer),
                compound_op: None,
                location: None,
            })
        }
        _ => unsafe { std::mem::transmute::<&Statement<'_>, &Statement<'_>>(stmt) }, // Safe: just changing lifetime annotation
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
        Expression::Identifier { name, .. } if name == var_name => unsafe {
            std::mem::transmute::<&Expression<'_>, &Expression<'_>>(replacement)
        },
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
            operand: replace_variable_in_expression(operand, var_name, replacement, optimizer),
            location: None,
        }),
        Expression::Index { object, index, .. } => optimizer.alloc_expr(Expression::Index {
            object: replace_variable_in_expression(object, var_name, replacement, optimizer),
            index: replace_variable_in_expression(index, var_name, replacement, optimizer),
            location: None,
        }),
        _ => unsafe { std::mem::transmute::<&Expression<'_>, &Expression<'_>>(expr) }, // Safe: just changing lifetime annotation
    }
}

/// Try to apply strength reduction to binary operations
pub(in crate::optimizer) fn try_strength_reduction<'ast>(
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
