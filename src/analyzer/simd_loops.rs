//! SIMD loop classification — detects a narrow set of vectorizable `for` loops.
//!
//! Used by Rust codegen (`simd_transform`) to emit `std::arch` SIMD blocks with
//! scalar tails. This module does **not** rewrite the AST; it only analyzes it.

use crate::parser::{BinaryOp, CompoundOp, Expression, Pattern, Statement};

/// Recognized SIMD-friendly loop shapes (Rust `f32` lanes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimdLoopPattern {
    /// `accum += slice_a[i] * slice_b[i]` with half-open `0..upper`.
    F32DotProductAccum {
        loop_var: String,
        accum_var: String,
        slice_a: String,
        slice_b: String,
    },
    /// `out[i] = slice_a[i] + slice_b[i]` with half-open `0..upper`.
    F32ArrayAddInto {
        loop_var: String,
        out_array: String,
        slice_a: String,
        slice_b: String,
    },
}

fn expr_identifier_name<'ast>(expr: &'ast Expression<'ast>) -> Option<&'ast str> {
    match expr {
        Expression::Identifier { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

fn is_zero_literal<'ast>(expr: &'ast Expression<'ast>) -> bool {
    matches!(
        expr,
        Expression::Literal { value, .. }
            if matches!(
                value,
                crate::parser::Literal::Int(0) | crate::parser::Literal::IntSuffixed(0, _)
            )
    )
}

/// Half-open numeric range starting at literal `0`, exclusive end `upper`.
pub fn half_open_range_from_zero<'ast>(
    iterable: &'ast Expression<'ast>,
) -> Option<&'ast Expression<'ast>> {
    match iterable {
        Expression::Range {
            start,
            end,
            inclusive,
            ..
        } if !inclusive && is_zero_literal(start) => Some(end),
        _ => None,
    }
}

fn index_with_loop_var<'ast>(expr: &'ast Expression<'ast>, loop_var: &str) -> Option<(String, ())> {
    match expr {
        Expression::Index { object, index, .. } => {
            if expr_identifier_name(index)? != loop_var {
                return None;
            }
            let arr = expr_identifier_name(object)?.to_string();
            Some((arr, ()))
        }
        _ => None,
    }
}

fn mul_of_two_indices<'ast>(
    expr: &'ast Expression<'ast>,
    loop_var: &str,
) -> Option<(String, String)> {
    match expr {
        Expression::Binary {
            op: BinaryOp::Mul,
            left,
            right,
            ..
        } => {
            let (a, ()) = index_with_loop_var(left, loop_var)?;
            let (b, ()) = index_with_loop_var(right, loop_var)?;
            Some((a, b))
        }
        _ => None,
    }
}

fn add_of_two_indices<'ast>(
    expr: &'ast Expression<'ast>,
    loop_var: &str,
) -> Option<(String, String)> {
    match expr {
        Expression::Binary {
            op: BinaryOp::Add,
            left,
            right,
            ..
        } => {
            let (a, ()) = index_with_loop_var(left, loop_var)?;
            let (b, ()) = index_with_loop_var(right, loop_var)?;
            Some((a, b))
        }
        _ => None,
    }
}

fn loop_body_safe_for_simd<'ast>(body: &[&'ast Statement<'ast>]) -> bool {
    if body.len() != 1 {
        return false;
    }
    match body[0] {
        Statement::Return { .. } | Statement::Break { .. } | Statement::Continue { .. } => false,
        Statement::If { .. }
        | Statement::Match { .. }
        | Statement::While { .. }
        | Statement::For { .. }
        | Statement::Loop { .. } => false,
        Statement::Assignment { target, value, .. } => {
            !expression_contains_call(target) && !expression_contains_call(value)
        }
        Statement::Expression { expr, .. } => !expression_contains_call(expr),
        _ => false,
    }
}

fn expression_contains_call(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Call { .. }
        | Expression::MethodCall { .. }
        | Expression::MacroInvocation { .. } => true,
        Expression::Binary { left, right, .. } => {
            expression_contains_call(left) || expression_contains_call(right)
        }
        Expression::Unary { operand, .. } => expression_contains_call(operand),
        Expression::Index { object, index, .. } => {
            expression_contains_call(object) || expression_contains_call(index)
        }
        Expression::Tuple { elements, .. } => elements.iter().any(|e| expression_contains_call(e)),
        Expression::Array { elements, .. } => elements.iter().any(|e| expression_contains_call(e)),
        Expression::Cast { expr, .. } => expression_contains_call(expr),
        Expression::TryOp { expr, .. } => expression_contains_call(expr),
        Expression::Await { expr, .. } => expression_contains_call(expr),
        Expression::FieldAccess { object, .. } => expression_contains_call(object),
        Expression::StructLiteral { fields, .. } => {
            fields.iter().any(|(_, e)| expression_contains_call(e))
        }
        Expression::MapLiteral { pairs, .. } => pairs
            .iter()
            .any(|(k, v)| expression_contains_call(k) || expression_contains_call(v)),
        Expression::Block { statements, .. } => {
            statements.iter().any(|s| statement_contains_call(s))
        }
        Expression::ChannelSend { channel, value, .. } => {
            expression_contains_call(channel) || expression_contains_call(value)
        }
        Expression::ChannelRecv { channel, .. } => expression_contains_call(channel),
        Expression::Closure { body, .. } => expression_contains_call(body),
        _ => false,
    }
}

fn statement_contains_call(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::Expression { expr, .. } => expression_contains_call(expr),
        Statement::Let {
            value, else_block, ..
        } => {
            expression_contains_call(value)
                || else_block
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| statement_contains_call(s)))
        }
        Statement::Assignment { target, value, .. } => {
            expression_contains_call(target) || expression_contains_call(value)
        }
        Statement::Return { value: Some(e), .. } => expression_contains_call(e),
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_contains_call(condition)
                || then_block.iter().any(|s| statement_contains_call(s))
                || else_block
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| statement_contains_call(s)))
        }
        _ => true,
    }
}

fn loop_binding_identifier<'a>(pattern: &'a Pattern<'a>) -> Option<&'a str> {
    match pattern {
        Pattern::Identifier(name) => Some(name.as_str()),
        _ => None,
    }
}

/// Analyze a `for` loop for SIMD codegen. Returns `None` if unsupported.
pub fn analyze_for_loop_simd<'ast>(
    pattern: &Pattern<'ast>,
    iterable: &'ast Expression<'ast>,
    body: &[&'ast Statement<'ast>],
) -> Option<SimdLoopPattern> {
    let loop_var = loop_binding_identifier(pattern)?.to_string();
    half_open_range_from_zero(iterable)?;
    if !loop_body_safe_for_simd(body) {
        return None;
    }

    match body[0] {
        Statement::Assignment {
            target,
            value,
            compound_op: Some(CompoundOp::Add),
            ..
        } => {
            if expression_contains_call(target) {
                return None;
            }
            let accum = expr_identifier_name(target)?.to_string();
            let (slice_a, slice_b) = mul_of_two_indices(value, &loop_var)?;
            Some(SimdLoopPattern::F32DotProductAccum {
                loop_var,
                accum_var: accum,
                slice_a,
                slice_b,
            })
        }
        Statement::Assignment {
            target,
            value,
            compound_op: None,
            ..
        } => {
            if expression_contains_call(target) || expression_contains_call(value) {
                return None;
            }
            let (out_arr, ()) = index_with_loop_var(target, &loop_var)?;
            let (slice_a, slice_b) = add_of_two_indices(value, &loop_var)?;
            Some(SimdLoopPattern::F32ArrayAddInto {
                loop_var,
                out_array: out_arr,
                slice_a,
                slice_b,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Literal, Pattern, Statement};
    use crate::test_utils::{test_alloc_expr, test_alloc_stmt};

    #[test]
    fn detects_dot_product_pattern() {
        let i_pat = Pattern::Identifier("i".to_string());
        let iterable = test_alloc_expr(Expression::Range {
            start: test_alloc_expr(Expression::Literal {
                value: Literal::Int(0),
                location: None,
            }),
            end: test_alloc_expr(Expression::MethodCall {
                object: test_alloc_expr(Expression::Identifier {
                    name: "a".to_string(),
                    location: None,
                }),
                method: "len".to_string(),
                type_args: None,
                arguments: vec![],
                location: None,
            }),
            inclusive: false,
            location: None,
        });
        let body_stmt = test_alloc_stmt(Statement::Assignment {
            target: test_alloc_expr(Expression::Identifier {
                name: "sum".to_string(),
                location: None,
            }),
            value: test_alloc_expr(Expression::Binary {
                left: test_alloc_expr(Expression::Index {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "a".to_string(),
                        location: None,
                    }),
                    index: test_alloc_expr(Expression::Identifier {
                        name: "i".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
                op: BinaryOp::Mul,
                right: test_alloc_expr(Expression::Index {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "b".to_string(),
                        location: None,
                    }),
                    index: test_alloc_expr(Expression::Identifier {
                        name: "i".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
                location: None,
            }),
            compound_op: Some(CompoundOp::Add),
            location: None,
        });
        let body: Vec<&Statement> = vec![&body_stmt];

        let pat = analyze_for_loop_simd(&i_pat, iterable, &body).expect("pattern");
        match pat {
            SimdLoopPattern::F32DotProductAccum {
                accum_var,
                slice_a,
                slice_b,
                ..
            } => {
                assert_eq!(accum_var, "sum");
                assert_eq!(slice_a, "a");
                assert_eq!(slice_b, "b");
            }
            other => panic!("unexpected pattern {:?}", other),
        }
    }

    #[test]
    fn detects_array_add_into_pattern() {
        let iterable = test_alloc_expr(Expression::Range {
            start: test_alloc_expr(Expression::Literal {
                value: Literal::Int(0),
                location: None,
            }),
            end: test_alloc_expr(Expression::MethodCall {
                object: test_alloc_expr(Expression::Identifier {
                    name: "out".to_string(),
                    location: None,
                }),
                method: "len".to_string(),
                type_args: None,
                arguments: vec![],
                location: None,
            }),
            inclusive: false,
            location: None,
        });
        let body_stmt = test_alloc_stmt(Statement::Assignment {
            target: test_alloc_expr(Expression::Index {
                object: test_alloc_expr(Expression::Identifier {
                    name: "out".to_string(),
                    location: None,
                }),
                index: test_alloc_expr(Expression::Identifier {
                    name: "i".to_string(),
                    location: None,
                }),
                location: None,
            }),
            value: test_alloc_expr(Expression::Binary {
                left: test_alloc_expr(Expression::Index {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "a".to_string(),
                        location: None,
                    }),
                    index: test_alloc_expr(Expression::Identifier {
                        name: "i".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
                op: BinaryOp::Add,
                right: test_alloc_expr(Expression::Index {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "b".to_string(),
                        location: None,
                    }),
                    index: test_alloc_expr(Expression::Identifier {
                        name: "i".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
                location: None,
            }),
            compound_op: None,
            location: None,
        });

        match analyze_for_loop_simd(
            &Pattern::Identifier("i".to_string()),
            iterable,
            &[&body_stmt],
        )
        .expect("pattern")
        {
            SimdLoopPattern::F32ArrayAddInto {
                out_array,
                slice_a,
                slice_b,
                ..
            } => {
                assert_eq!(out_array, "out");
                assert_eq!(slice_a, "a");
                assert_eq!(slice_b, "b");
            }
            other => panic!("unexpected pattern {:?}", other),
        }
    }

    #[test]
    fn rejects_nonempty_side_effect_body() {
        let iterable = test_alloc_expr(Expression::Range {
            start: test_alloc_expr(Expression::Literal {
                value: Literal::Int(0),
                location: None,
            }),
            end: test_alloc_expr(Expression::Literal {
                value: Literal::Int(10),
                location: None,
            }),
            inclusive: false,
            location: None,
        });
        let bad = test_alloc_stmt(Statement::Expression {
            expr: test_alloc_expr(Expression::Call {
                function: test_alloc_expr(Expression::Identifier {
                    name: "foo".to_string(),
                    location: None,
                }),
                arguments: vec![],
                location: None,
            }),
            location: None,
        });
        let body = vec![bad];
        assert!(
            analyze_for_loop_simd(&Pattern::Identifier("i".to_string()), iterable, &body).is_none()
        );
    }
}
