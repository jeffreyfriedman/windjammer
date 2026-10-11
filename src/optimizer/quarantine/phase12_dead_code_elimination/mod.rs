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

mod control_flow;
mod eliminate;
mod liveness;

use crate::parser::{FunctionDecl, Item, Program};

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
    let called_functions = liveness::find_called_functions(program);

    // Step 2: Process all items, removing dead code
    let mut new_items = Vec::new();
    for item in &program.items {
        match item {
            Item::Function {
                decl: func,
                location,
            } => {
                // Check if function is unused (private and never called)
                if liveness::is_unused_function(func, &called_functions) {
                    stats.unused_functions_removed += 1;
                    continue; // Skip this function
                }

                // Process function body to remove dead code
                let (new_body, func_stats) =
                    eliminate::eliminate_dead_code_in_statements(&func.body, optimizer);
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
                let new_impl =
                    eliminate::eliminate_dead_code_in_impl(impl_block, &mut stats, optimizer);
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
                let new_value = eliminate::eliminate_dead_code_in_expression(value, optimizer);
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
                is_pub,
                location,
            } => {
                // Process const initializers
                let new_value = eliminate::eliminate_dead_code_in_expression(value, optimizer);
                new_items.push(Item::Const {
                    name: name.clone(),
                    type_: type_.clone(),
                    value: new_value,
                    is_pub: *is_pub,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{
        Decorator, Expression, FunctionDecl, Item, Literal, Pattern, Statement, Type,
    };
    use crate::test_utils::{test_alloc_expr, test_alloc_stmt};

    fn make_pub_func<'ast>(name: &str, body: Vec<&'ast Statement<'ast>>) -> FunctionDecl<'ast> {
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
            return_decorators: vec![],
            body,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
        }
    }

    fn make_private_func<'ast>(name: &str, body: Vec<&'ast Statement<'ast>>) -> FunctionDecl<'ast> {
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
            return_decorators: vec![],
            body,
            parent_type: None,
            impl_trait: None,
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
                        test_alloc_stmt(Statement::Return {
                            value: Some(test_alloc_expr(Expression::Literal {
                                value: Literal::Int(42),
                                location: None,
                            })),
                            location: None,
                        }),
                        test_alloc_stmt(Statement::Expression {
                            expr: test_alloc_expr(Expression::MacroInvocation {
                                name: "println".to_string(),
                                args: vec![test_alloc_expr(Expression::Literal {
                                    value: Literal::String("unreachable".to_string()),
                                    location: None,
                                })],
                                delimiter: crate::parser::MacroDelimiter::Parens,
                                is_repeat: false,
                                location: None,
                            }),
                            location: None,
                        }),
                    ],
                ),
                location: None,
            }],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (optimized, stats) = eliminate_dead_code(&program, &optimizer);
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
                        vec![test_alloc_stmt(Statement::Return {
                            value: None,
                            location: None,
                        })],
                    ),
                    location: None,
                },
                Item::Function {
                    decl: make_private_func(
                        "unused_helper",
                        vec![test_alloc_stmt(Statement::Return {
                            value: None,
                            location: None,
                        })],
                    ),
                    location: None,
                },
            ],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (optimized, stats) = eliminate_dead_code(&program, &optimizer);
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
                        vec![test_alloc_stmt(Statement::Expression {
                            expr: test_alloc_expr(Expression::Call {
                                function: test_alloc_expr(Expression::Identifier {
                                    name: "helper".to_string(),
                                    location: None,
                                }),
                                arguments: vec![],
                                location: None,
                            }),
                            location: None,
                        })],
                    ),
                    location: None,
                },
                Item::Function {
                    decl: make_private_func(
                        "helper",
                        vec![test_alloc_stmt(Statement::Return {
                            value: None,
                            location: None,
                        })],
                    ),
                    location: None,
                },
            ],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (optimized, stats) = eliminate_dead_code(&program, &optimizer);
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
                        test_alloc_stmt(Statement::If {
                            condition: test_alloc_expr(Expression::Literal {
                                value: Literal::Bool(true),
                                location: None,
                            }),
                            then_block: vec![],
                            else_block: Some(vec![]),
                            location: None,
                        }),
                        test_alloc_stmt(Statement::Return {
                            value: None,
                            location: None,
                        }),
                    ],
                ),
                location: None,
            }],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (optimized, stats) = eliminate_dead_code(&program, &optimizer);
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
                    vec![test_alloc_stmt(Statement::If {
                        condition: test_alloc_expr(Expression::Literal {
                            value: Literal::Bool(true),
                            location: None,
                        }),
                        then_block: vec![
                            test_alloc_stmt(Statement::Return {
                                value: Some(test_alloc_expr(Expression::Literal {
                                    value: Literal::Int(1),
                                    location: None,
                                })),
                                location: None,
                            }),
                            test_alloc_stmt(Statement::Expression {
                                expr: test_alloc_expr(Expression::Literal {
                                    value: Literal::Int(2),
                                    location: None,
                                }),
                                location: None,
                            }),
                        ],
                        else_block: Some(vec![
                            test_alloc_stmt(Statement::Return {
                                value: Some(test_alloc_expr(Expression::Literal {
                                    value: Literal::Int(3),
                                    location: None,
                                })),
                                location: None,
                            }),
                            test_alloc_stmt(Statement::Expression {
                                expr: test_alloc_expr(Expression::Literal {
                                    value: Literal::Int(4),
                                    location: None,
                                }),
                                location: None,
                            }),
                        ]),
                        location: None,
                    })],
                ),
                location: None,
            }],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (optimized, stats) = eliminate_dead_code(&program, &optimizer);
        assert_eq!(stats.unreachable_statements_removed, 2);

        let func = match &optimized.items[0] {
            Item::Function { decl, .. } => decl,
            _ => panic!("Expected function"),
        };
        if let Statement::If {
            then_block,
            else_block,
            ..
        } = func.body[0]
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
                        test_alloc_stmt(Statement::Return {
                            value: Some(test_alloc_expr(Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            })),
                            location: None,
                        }),
                    ],
                ),
                location: None,
            }],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let (_, stats) = eliminate_dead_code(&program, &optimizer);
        assert_eq!(stats.unreachable_statements_removed, 0);
        assert_eq!(stats.unused_functions_removed, 0);
        assert_eq!(stats.empty_blocks_removed, 0);
    }
}
