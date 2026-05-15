//! Phase 11: String Interning
//!
//! **What It Does:**
//! Deduplicates string literals by creating a global string pool and replacing
//! duplicate literals with references to the pool. This reduces memory usage
//! and improves cache locality.
//!
//! **Example:**
//! ```windjammer
//! // Before:
//! fn greet_alice() { println!("Hello") }
//! fn greet_bob() { println!("Hello") }
//! fn greet_charlie() { println!("Hello") }
//!
//! // After:
//! static __STRING_POOL_0: &str = "Hello";
//! fn greet_alice() { println!(__STRING_POOL_0) }
//! fn greet_bob() { println!(__STRING_POOL_0) }
//! fn greet_charlie() { println!(__STRING_POOL_0) }
//! ```
//!
//! **Benefits:**
//! - Reduces binary size (fewer duplicate strings)
//! - Improves memory usage (shared string data)
//! - Better cache locality (fewer string copies)
//! - Typical savings: 5-20% of string data
//!
//! **When Applied:**
//! - String literal appears 2+ times
//! - String length >= 10 characters (threshold)
//! - Not applied to format strings or interpolated strings

mod analysis;
mod pool;
mod replace;

use crate::parser::{Item, Program};

/// Result of string interning optimization
#[derive(Debug, Clone)]
pub struct StringInterningResult<'ast> {
    pub program: Program<'ast>,
    pub strings_interned: usize,
    pub memory_saved: usize,
}

/// Main optimization function
pub fn optimize_string_interning<'ast>(
    program: &Program<'ast>,
    optimizer: &crate::optimizer::Optimizer,
) -> StringInterningResult<'ast> {
    use pool::{build_string_pool, create_pool_map, create_pool_statics};

    // Step 1: Analyze string literals
    let frequency = analysis::analyze_string_literals(program);

    // Step 2: Build string pool
    let pool = build_string_pool(frequency);

    // Calculate statistics
    let strings_interned = pool.len();
    let memory_saved: usize = pool
        .iter()
        .map(|entry| entry.value.len() * (entry.count - 1)) // -1 because one copy remains
        .sum();

    // Step 3: Create pool map for lookups
    let pool_map = create_pool_map(&pool);

    // Step 4: Create static declarations
    let pool_statics = create_pool_statics(&pool, optimizer);

    // Step 5: Transform program items
    let transformed_items: Vec<Item<'ast>> = program
        .items
        .iter()
        .map(|item| replace::replace_strings_in_item(item, &pool_map, optimizer))
        .collect();

    // Step 6: Combine pool statics + transformed items
    let mut new_items = pool_statics;
    new_items.extend(transformed_items);

    StringInterningResult {
        program: Program { items: new_items },
        strings_interned,
        memory_saved,
    }
}

#[cfg(test)]
mod tests {
    use super::analysis::analyze_string_literals;
    use super::*;
    use crate::parser::*;
    use crate::test_utils::{test_alloc_expr, test_alloc_stmt};

    fn create_test_function<'ast>(
        name: &str,
        body_stmts: Vec<&'ast Statement<'ast>>,
    ) -> Item<'ast> {
        Item::Function {
            decl: FunctionDecl {
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
                body: body_stmts,
                parent_type: None,
                impl_trait: None,
                doc_comment: None,
            },
            location: None,
        }
    }

    #[test]
    fn test_string_frequency_analysis() {
        let program = Program {
            items: vec![
                create_test_function(
                    "test1",
                    vec![test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        }),
                        location: None,
                    })],
                ),
                create_test_function(
                    "test2",
                    vec![test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        }),
                        location: None,
                    })],
                ),
            ],
        };

        let frequency = analyze_string_literals(&program);
        assert_eq!(frequency.get("Hello World"), Some(&2));
    }

    #[test]
    fn test_full_transformation() {
        let program = Program {
            items: vec![
                create_test_function(
                    "test1",
                    vec![test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        }),
                        location: None,
                    })],
                ),
                create_test_function(
                    "test2",
                    vec![test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        }),
                        location: None,
                    })],
                ),
            ],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let result = optimize_string_interning(&program, &optimizer);

        // Should have 3 items: 1 static + 2 functions
        assert_eq!(result.program.items.len(), 3);

        // First item should be the string pool static
        match &result.program.items[0] {
            Item::Static { name, value, .. } => {
                assert_eq!(name, "__STRING_POOL_0");
                if let Expression::Literal {
                    value: Literal::String(s),
                    ..
                } = value
                {
                    assert_eq!(s, "Hello World");
                } else {
                    panic!("Expected string literal");
                }
            }
            _ => panic!("Expected static declaration"),
        }

        // Functions should reference the pool
        match &result.program.items[1] {
            Item::Function { decl: f, .. } => {
                if let Some(stmt) = f.body.first() {
                    if let Statement::Expression { expr, .. } = stmt {
                        if let Expression::Identifier { name, .. } = expr {
                            assert_eq!(name, "__STRING_POOL_0");
                        } else {
                            panic!("Expected identifier");
                        }
                    } else {
                        panic!("Expected expression statement");
                    }
                } else {
                    panic!("Expected statement");
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_memory_savings_calculation() {
        let program = Program {
            items: vec![create_test_function(
                "test1",
                vec![
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                ],
            )],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let result = optimize_string_interning(&program, &optimizer);

        // "Hello World" = 11 bytes, appears 3 times, saves 2 copies = 22 bytes
        assert_eq!(result.strings_interned, 1);
        assert_eq!(result.memory_saved, 22);
    }

    #[test]
    fn test_minimum_length_threshold() {
        let program = Program {
            items: vec![create_test_function(
                "test",
                vec![
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hi".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hi".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Hi".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                ],
            )],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let result = optimize_string_interning(&program, &optimizer);

        // Should not intern short strings (< 10 chars)
        assert_eq!(result.strings_interned, 0);
        assert_eq!(result.memory_saved, 0);
    }

    #[test]
    fn test_nested_expressions() {
        let program = Program {
            items: vec![create_test_function(
                "test",
                vec![test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::Binary {
                        left: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Long String Value".to_string()),
                            location: None,
                        }),
                        op: BinaryOp::Add,
                        right: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Long String Value".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                    location: None,
                })],
            )],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let result = optimize_string_interning(&program, &optimizer);
        assert_eq!(result.strings_interned, 1);
        assert_eq!(result.memory_saved, 17); // "Long String Value" = 17 bytes

        // Check transformation
        match &result.program.items[1] {
            Item::Function { decl: f, .. } => {
                if let Some(stmt) = f.body.first() {
                    if let Statement::Expression { expr, .. } = stmt {
                        if let Expression::Binary { left, right, .. } = expr {
                            // Both sides should reference the pool
                            assert!(matches!(left, Expression::Identifier { .. }));
                            assert!(matches!(right, Expression::Identifier { .. }));
                        } else {
                            panic!("Expected binary expression");
                        }
                    } else {
                        panic!("Expected expression statement");
                    }
                } else {
                    panic!("Expected statement");
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_multiple_different_strings() {
        let program = Program {
            items: vec![create_test_function(
                "test",
                vec![
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("First String".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("First String".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Second String".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                    test_alloc_stmt(Statement::Expression {
                        expr: test_alloc_expr(Expression::Literal {
                            value: Literal::String("Second String".to_string()),
                            location: None,
                        }),
                        location: None,
                    }),
                ],
            )],
        };

        let optimizer = crate::optimizer::Optimizer::with_defaults();
        let result = optimize_string_interning(&program, &optimizer);

        // Should intern both strings
        assert_eq!(result.strings_interned, 2);
        // "First String" = 12 bytes, "Second String" = 13 bytes
        // Total savings = 12 + 13 = 25 bytes
        assert_eq!(result.memory_saved, 25);

        // Should have 3 items: 2 statics + 1 function
        assert_eq!(result.program.items.len(), 3);
    }
}
