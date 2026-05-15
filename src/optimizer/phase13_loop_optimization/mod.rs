#![allow(clippy::collapsible_match)]

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

mod loop_analysis;
mod loop_invariant_motion;
mod loop_transformations;
mod loop_walk;

use crate::parser::{FunctionDecl, ImplBlock, Item, Program};

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
pub fn optimize_loops<'ast>(
    program: &Program<'ast>,
    optimizer: &crate::optimizer::Optimizer,
) -> (Program<'ast>, LoopOptimizationStats) {
    optimize_loops_with_config(program, &LoopOptimizationConfig::default(), optimizer)
}

/// Optimize loops with custom configuration
pub fn optimize_loops_with_config<'ast>(
    program: &Program<'ast>,
    config: &LoopOptimizationConfig,
    optimizer: &crate::optimizer::Optimizer,
) -> (Program<'ast>, LoopOptimizationStats) {
    let mut stats = LoopOptimizationStats::default();

    let new_items = program
        .items
        .iter()
        .map(|item| optimize_loops_in_item(item, config, &mut stats, optimizer))
        .collect();

    (
        unsafe { std::mem::transmute::<Program<'_>, Program<'_>>(Program { items: new_items }) },
        stats,
    )
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
            let new_body =
                loop_walk::optimize_loops_in_statements(&func.body, config, stats, optimizer);
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
                    return_decorators: func.return_decorators.clone(),
                    body: new_body,
                    parent_type: func.parent_type.clone(),
                    impl_trait: func.impl_trait.clone(),
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
                    return_decorators: func.return_decorators.clone(),
                    body: loop_walk::optimize_loops_in_statements(
                        &func.body, config, stats, optimizer,
                    ),
                    parent_type: func.parent_type.clone(),
                    impl_trait: func.impl_trait.clone(),
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
                    is_extern: impl_block.is_extern,
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
            value: loop_walk::optimize_loops_in_expression(value, config, stats, optimizer),
            location: location.clone(),
        },
        Item::Const {
            name,
            type_,
            value,
            is_pub,
            location,
        } => Item::Const {
            name: name.clone(),
            type_: type_.clone(),
            value: loop_walk::optimize_loops_in_expression(value, config, stats, optimizer),
            is_pub: *is_pub,
            location: location.clone(),
        },
        _ => item.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{BinaryOp, Decorator, Literal, Pattern, Type};
    use crate::parser_impl::{Expression, Statement};
    use crate::test_utils::{test_alloc_expr, test_alloc_stmt};

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
                    impl_trait: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: None,
                    return_decorators: Vec::new(),
                    body: vec![test_alloc_stmt(Statement::For {
                        pattern: Pattern::Identifier("i".to_string()),
                        iterable: test_alloc_expr(Expression::Range {
                            start: test_alloc_expr(Expression::Literal {
                                value: Literal::Int(0),
                                location: None,
                            }),
                            end: test_alloc_expr(Expression::Literal {
                                value: Literal::Int(3),
                                location: None,
                            }),
                            inclusive: false,
                            location: None,
                        }),
                        body: vec![test_alloc_stmt(Statement::Expression {
                            expr: test_alloc_expr(Expression::MacroInvocation {
                                name: "println".to_string(),
                                args: vec![test_alloc_expr(Expression::Identifier {
                                    name: "i".to_string(),
                                    location: None,
                                })],
                                delimiter: crate::parser::MacroDelimiter::Parens,
                                is_repeat: false,
                                location: None,
                            }),
                            location: None,
                        })],
                        location: None,
                    })],
                },
                location: None,
            }],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (optimized, stats) = optimize_loops(&program, &optimizer);
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
                    impl_trait: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: None,
                    return_decorators: Vec::new(),
                    body: vec![test_alloc_stmt(Statement::For {
                        pattern: Pattern::Identifier("i".to_string()),
                        iterable: test_alloc_expr(Expression::Range {
                            start: test_alloc_expr(Expression::Literal {
                                value: Literal::Int(0),
                                location: None,
                            }),
                            end: test_alloc_expr(Expression::Literal {
                                value: Literal::Int(100),
                                location: None,
                            }),
                            inclusive: false,
                            location: None,
                        }),
                        body: vec![
                            // Loop-invariant: doesn't use 'i'
                            test_alloc_stmt(Statement::Let {
                                pattern: Pattern::Identifier("x".to_string()),
                                mutable: false,
                                type_: None,
                                value: test_alloc_expr(Expression::Literal {
                                    value: Literal::Int(42),
                                    location: None,
                                }),
                                else_block: None,
                                location: None,
                            }),
                            // Loop-variant: uses 'i'
                            test_alloc_stmt(Statement::Expression {
                                expr: test_alloc_expr(Expression::Binary {
                                    left: test_alloc_expr(Expression::Identifier {
                                        name: "x".to_string(),
                                        location: None,
                                    }),
                                    op: BinaryOp::Add,
                                    right: test_alloc_expr(Expression::Identifier {
                                        name: "i".to_string(),
                                        location: None,
                                    }),
                                    location: None,
                                }),
                                location: None,
                            }),
                        ],
                        location: None,
                    })],
                },
                location: None,
            }],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (optimized, stats) = optimize_loops(&program, &optimizer);
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
                    impl_trait: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: Some(Type::Custom("i32".to_string())),
                    return_decorators: Vec::new(),
                    body: vec![test_alloc_stmt(Statement::Return {
                        value: Some(test_alloc_expr(Expression::Binary {
                            left: test_alloc_expr(Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            }),
                            op: BinaryOp::Mul,
                            right: test_alloc_expr(Expression::Literal {
                                value: Literal::Int(4),
                                location: None,
                            }),
                            location: None,
                        })),
                        location: None,
                    })],
                },
                location: None,
            }],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (_, stats) = optimize_loops(&program, &optimizer);
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
                    impl_trait: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: None,
                    return_decorators: Vec::new(),
                    body: vec![test_alloc_stmt(Statement::For {
                        pattern: Pattern::Identifier("i".to_string()),
                        iterable: test_alloc_expr(Expression::Range {
                            start: test_alloc_expr(Expression::Literal {
                                value: Literal::Int(0),
                                location: None,
                            }),
                            end: test_alloc_expr(Expression::Literal {
                                value: Literal::Int(1000),
                                location: None,
                            }), // Too large
                            inclusive: false,
                            location: None,
                        }),
                        body: vec![test_alloc_stmt(Statement::Expression {
                            expr: test_alloc_expr(Expression::Identifier {
                                name: "i".to_string(),
                                location: None,
                            }),
                            location: None,
                        })],
                        location: None,
                    })],
                },
                location: None,
            }],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (optimized, stats) = optimize_loops(&program, &optimizer);
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
                    impl_trait: None,
                    doc_comment: None,
                    parameters: vec![],
                    return_type: None,
                    return_decorators: Vec::new(),
                    body: vec![test_alloc_stmt(Statement::For {
                        pattern: Pattern::Identifier("i".to_string()),
                        iterable: test_alloc_expr(Expression::Range {
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
                        }),
                        body: vec![
                            // Loop-variant: uses 'i'
                            test_alloc_stmt(Statement::Let {
                                pattern: Pattern::Identifier("x".to_string()),
                                mutable: false,
                                type_: None,
                                value: test_alloc_expr(Expression::Binary {
                                    left: test_alloc_expr(Expression::Identifier {
                                        name: "i".to_string(),
                                        location: None,
                                    }),
                                    op: BinaryOp::Mul,
                                    right: test_alloc_expr(Expression::Literal {
                                        value: Literal::Int(2),
                                        location: None,
                                    }),
                                    location: None,
                                }),
                                else_block: None,
                                location: None,
                            }),
                        ],
                        location: None,
                    })],
                },
                location: None,
            }],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (_, stats) = optimize_loops(&program, &optimizer);
        assert_eq!(stats.invariants_hoisted, 0); // Should not hoist
    }
}
