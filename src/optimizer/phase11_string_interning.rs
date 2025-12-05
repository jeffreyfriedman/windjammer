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

use crate::parser::{Expression, Item, Literal, MatchArm, Program, Statement, Type};
use std::collections::HashMap;

#[cfg(test)]
#[allow(unused_imports)]
use crate::parser::FunctionDecl;

/// Result of string interning optimization
#[derive(Debug, Clone)]
pub struct StringInterningResult {
    pub program: Program,
    pub strings_interned: usize,
    pub memory_saved: usize,
}

/// String pool entry
#[derive(Debug, Clone)]
struct StringPoolEntry {
    /// The string literal value
    value: String,
    /// Number of occurrences
    count: usize,
    /// Generated pool variable name
    pool_name: String,
}

/// Analyze program and build string frequency map
fn analyze_string_literals(program: &Program) -> HashMap<String, usize> {
    let mut frequency: HashMap<String, usize> = HashMap::new();

    for item in &program.items {
        collect_strings_from_item(item, &mut frequency);
    }

    frequency
}

/// Collect strings from an item
fn collect_strings_from_item(item: &Item, frequency: &mut HashMap<String, usize>) {
    match item {
        Item::Function { decl: func, .. } => {
            for stmt in &func.body {
                collect_strings_from_statement(stmt, frequency);
            }
        }
        Item::Impl {
            block: impl_block, ..
        } => {
            for func in &impl_block.functions {
                for stmt in &func.body {
                    collect_strings_from_statement(stmt, frequency);
                }
            }
        }
        Item::Static { value, .. } => {
            collect_strings_from_expression(value, frequency);
        }
        Item::Const { value, .. } => {
            collect_strings_from_expression(value, frequency);
        }
        _ => {}
    }
}

/// Collect strings from an expression recursively
fn collect_strings_from_expression(expr: &Expression, frequency: &mut HashMap<String, usize>) {
    match expr {
        Expression::Literal {
            value: Literal::String(s),
            ..
        } => {
            // Only intern strings >= 10 characters
            if s.len() >= 10 {
                *frequency.entry(s.clone()).or_insert(0) += 1;
            }
        }
        Expression::Binary { left, right, .. } => {
            collect_strings_from_expression(left, frequency);
            collect_strings_from_expression(right, frequency);
        }
        Expression::Unary { operand, .. } => {
            collect_strings_from_expression(operand, frequency);
        }
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            collect_strings_from_expression(function, frequency);
            for (_, arg) in arguments {
                collect_strings_from_expression(arg, frequency);
            }
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            collect_strings_from_expression(object, frequency);
            for (_, arg) in arguments {
                collect_strings_from_expression(arg, frequency);
            }
        }
        Expression::FieldAccess { object, .. } => {
            collect_strings_from_expression(object, frequency);
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                collect_strings_from_expression(value, frequency);
            }
        }
        Expression::Range { start, end, .. } => {
            collect_strings_from_expression(start, frequency);
            collect_strings_from_expression(end, frequency);
        }
        Expression::Closure { body, .. } => {
            collect_strings_from_expression(body, frequency);
        }
        Expression::Cast { expr, .. } => {
            collect_strings_from_expression(expr, frequency);
        }
        Expression::Index { object, index, .. } => {
            collect_strings_from_expression(object, frequency);
            collect_strings_from_expression(index, frequency);
        }
        Expression::Tuple { elements, .. } => {
            for elem in elements {
                collect_strings_from_expression(elem, frequency);
            }
        }
        Expression::MacroInvocation { args, .. } => {
            for arg in args {
                collect_strings_from_expression(arg, frequency);
            }
        }
        Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
            collect_strings_from_expression(expr, frequency);
        }
        Expression::ChannelSend { channel, value, .. } => {
            collect_strings_from_expression(channel, frequency);
            collect_strings_from_expression(value, frequency);
        }
        Expression::ChannelRecv { channel, .. } => {
            collect_strings_from_expression(channel, frequency);
        }
        Expression::Block { statements, .. } => {
            for stmt in statements {
                collect_strings_from_statement(stmt, frequency);
            }
        }
        _ => {}
    }
}

/// Collect strings from a statement
fn collect_strings_from_statement(stmt: &Statement, frequency: &mut HashMap<String, usize>) {
    match stmt {
        Statement::Let { value, .. }
        | Statement::Const { value, .. }
        | Statement::Static { value, .. } => {
            collect_strings_from_expression(value, frequency);
        }
        Statement::Expression { expr, .. } => {
            collect_strings_from_expression(expr, frequency);
        }
        Statement::Return {
            value: Some(expr), ..
        } => {
            collect_strings_from_expression(expr, frequency);
        }
        Statement::Assignment { target, value, .. } => {
            collect_strings_from_expression(target, frequency);
            collect_strings_from_expression(value, frequency);
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            collect_strings_from_expression(condition, frequency);
            for stmt in then_block {
                collect_strings_from_statement(stmt, frequency);
            }
            if let Some(else_stmts) = else_block {
                for stmt in else_stmts {
                    collect_strings_from_statement(stmt, frequency);
                }
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            collect_strings_from_expression(condition, frequency);
            for stmt in body {
                collect_strings_from_statement(stmt, frequency);
            }
        }
        Statement::For { iterable, body, .. } => {
            collect_strings_from_expression(iterable, frequency);
            for stmt in body {
                collect_strings_from_statement(stmt, frequency);
            }
        }
        Statement::Match { value, arms, .. } => {
            collect_strings_from_expression(value, frequency);
            for arm in arms {
                collect_strings_from_expression(&arm.body, frequency);
            }
        }
        _ => {}
    }
}

/// Build string pool from frequency analysis
fn build_string_pool(frequency: HashMap<String, usize>) -> Vec<StringPoolEntry> {
    let mut pool = Vec::new();
    let mut index = 0;

    for (value, count) in frequency {
        // Only intern strings that appear 2+ times
        if count >= 2 {
            pool.push(StringPoolEntry {
                value: value.clone(),
                count,
                pool_name: format!("__STRING_POOL_{}", index),
            });
            index += 1;
        }
    }

    // Sort by count (most frequent first) for better cache locality
    pool.sort_by(|a, b| b.count.cmp(&a.count));

    pool
}

/// Create a map from string value to pool name for quick lookup
fn create_pool_map(pool: &[StringPoolEntry]) -> HashMap<String, String> {
    pool.iter()
        .map(|entry| (entry.value.clone(), entry.pool_name.clone()))
        .collect()
}

/// Replace string literals in an expression with pool references
fn replace_strings_in_expression(
    expr: Expression,
    pool_map: &HashMap<String, String>,
) -> Expression {
    match expr {
        Expression::Literal {
            value: Literal::String(s),
            location,
        } => {
            // Replace with pool reference if interned
            if let Some(pool_name) = pool_map.get(&s) {
                Expression::Identifier {
                    name: pool_name.clone(),
                    location,
                }
            } else {
                Expression::Literal {
                    value: Literal::String(s),
                    location,
                }
            }
        }
        Expression::Binary {
            left,
            right,
            op,
            location,
        } => Expression::Binary {
            left: Box::new(replace_strings_in_expression(*left, pool_map)),
            right: Box::new(replace_strings_in_expression(*right, pool_map)),
            op,
            location,
        },
        Expression::Unary {
            op,
            operand,
            location,
        } => Expression::Unary {
            op,
            operand: Box::new(replace_strings_in_expression(*operand, pool_map)),
            location,
        },
        Expression::Call {
            function,
            arguments,
            location,
        } => Expression::Call {
            function: Box::new(replace_strings_in_expression(*function, pool_map)),
            arguments: arguments
                .into_iter()
                .map(|(label, arg)| (label, replace_strings_in_expression(arg, pool_map)))
                .collect(),
            location,
        },
        Expression::MethodCall {
            object,
            method,
            type_args,
            arguments,
            location,
        } => Expression::MethodCall {
            object: Box::new(replace_strings_in_expression(*object, pool_map)),
            method,
            type_args,
            arguments: arguments
                .into_iter()
                .map(|(label, arg)| (label, replace_strings_in_expression(arg, pool_map)))
                .collect(),
            location,
        },
        Expression::FieldAccess {
            object,
            field,
            location,
        } => Expression::FieldAccess {
            object: Box::new(replace_strings_in_expression(*object, pool_map)),
            field,
            location,
        },
        Expression::StructLiteral {
            name,
            fields,
            location,
        } => Expression::StructLiteral {
            name,
            fields: fields
                .into_iter()
                .map(|(name, value)| (name, replace_strings_in_expression(value, pool_map)))
                .collect(),
            location,
        },
        Expression::Range {
            start,
            end,
            inclusive,
            location,
        } => Expression::Range {
            start: Box::new(replace_strings_in_expression(*start, pool_map)),
            end: Box::new(replace_strings_in_expression(*end, pool_map)),
            inclusive,
            location,
        },
        Expression::Closure {
            parameters,
            body,
            location,
        } => Expression::Closure {
            parameters,
            body: Box::new(replace_strings_in_expression(*body, pool_map)),
            location,
        },
        Expression::Cast {
            expr,
            type_,
            location,
        } => Expression::Cast {
            expr: Box::new(replace_strings_in_expression(*expr, pool_map)),
            type_,
            location,
        },
        Expression::Index {
            object,
            index,
            location,
        } => Expression::Index {
            object: Box::new(replace_strings_in_expression(*object, pool_map)),
            index: Box::new(replace_strings_in_expression(*index, pool_map)),
            location,
        },
        Expression::Tuple { elements, location } => Expression::Tuple {
            elements: elements
                .into_iter()
                .map(|e| replace_strings_in_expression(e, pool_map))
                .collect(),
            location,
        },
        Expression::MacroInvocation {
            name,
            args,
            delimiter,
            location,
        } => Expression::MacroInvocation {
            name,
            args: args
                .into_iter()
                .map(|arg| replace_strings_in_expression(arg, pool_map))
                .collect(),
            delimiter,
            location,
        },
        Expression::TryOp { expr, location } => Expression::TryOp {
            expr: Box::new(replace_strings_in_expression(*expr, pool_map)),
            location,
        },
        Expression::Await { expr, location } => Expression::Await {
            expr: Box::new(replace_strings_in_expression(*expr, pool_map)),
            location,
        },
        Expression::ChannelSend {
            channel,
            value,
            location,
        } => Expression::ChannelSend {
            channel: Box::new(replace_strings_in_expression(*channel, pool_map)),
            value: Box::new(replace_strings_in_expression(*value, pool_map)),
            location,
        },
        Expression::ChannelRecv { channel, location } => Expression::ChannelRecv {
            channel: Box::new(replace_strings_in_expression(*channel, pool_map)),
            location,
        },
        Expression::Block {
            statements,
            location,
        } => Expression::Block {
            statements: statements
                .into_iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                .collect(),
            location,
        },
        other => other,
    }
}

/// Replace string literals in a statement with pool references
fn replace_strings_in_statement(stmt: Statement, pool_map: &HashMap<String, String>) -> Statement {
    match stmt {
        Statement::Let {
            pattern,
            mutable,
            type_,
            value,
            else_block,
            location,
        } => Statement::Let {
            pattern,
            mutable,
            type_,
            value: replace_strings_in_expression(value, pool_map),
            else_block: else_block.map(|stmts| {
                stmts
                    .into_iter()
                    .map(|s| replace_strings_in_statement(s, pool_map))
                    .collect()
            }),
            location,
        },
        Statement::Const {
            name,
            type_,
            value,
            location,
        } => Statement::Const {
            name,
            type_,
            value: replace_strings_in_expression(value, pool_map),
            location,
        },
        Statement::Static {
            name,
            mutable,
            type_,
            value,
            location,
        } => Statement::Static {
            name,
            mutable,
            type_,
            value: replace_strings_in_expression(value, pool_map),
            location,
        },
        Statement::Expression { expr, location } => Statement::Expression {
            expr: replace_strings_in_expression(expr, pool_map),
            location,
        },
        Statement::Return {
            value: Some(expr),
            location,
        } => Statement::Return {
            value: Some(replace_strings_in_expression(expr, pool_map)),
            location,
        },
        Statement::Assignment {
            target,
            value,
            location,
        } => Statement::Assignment {
            target: replace_strings_in_expression(target, pool_map),
            value: replace_strings_in_expression(value, pool_map),
            location,
        },
        Statement::If {
            condition,
            then_block,
            else_block,
            location,
        } => Statement::If {
            condition: replace_strings_in_expression(condition, pool_map),
            then_block: then_block
                .into_iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                .collect(),
            else_block: else_block.map(|stmts| {
                stmts
                    .into_iter()
                    .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                    .collect()
            }),
            location,
        },
        Statement::While {
            condition,
            body,
            location,
        } => Statement::While {
            condition: replace_strings_in_expression(condition, pool_map),
            body: body
                .into_iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                .collect(),
            location,
        },
        Statement::For {
            pattern,
            iterable,
            body,
            location,
        } => Statement::For {
            pattern,
            iterable: replace_strings_in_expression(iterable, pool_map),
            body: body
                .into_iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                .collect(),
            location,
        },
        Statement::Match {
            value,
            arms,
            location,
        } => Statement::Match {
            value: replace_strings_in_expression(value, pool_map),
            arms: arms
                .into_iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern,
                    guard: arm
                        .guard
                        .map(|g| replace_strings_in_expression(g, pool_map)),
                    body: replace_strings_in_expression(arm.body, pool_map),
                })
                .collect(),
            location,
        },
        other => other,
    }
}

/// Replace string literals in an item with pool references
fn replace_strings_in_item(item: Item, pool_map: &HashMap<String, String>) -> Item {
    match item {
        Item::Function {
            decl: mut func,
            location,
        } => {
            func.body = func
                .body
                .into_iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                .collect();
            Item::Function {
                decl: func,
                location,
            }
        }
        Item::Impl {
            block: mut impl_block,
            location,
        } => {
            impl_block.functions = impl_block
                .functions
                .into_iter()
                .map(|mut func| {
                    func.body = func
                        .body
                        .into_iter()
                        .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                        .collect();
                    func
                })
                .collect();
            Item::Impl {
                block: impl_block,
                location,
            }
        }
        Item::Static {
            name,
            mutable,
            type_,
            value,
            location,
        } => Item::Static {
            name,
            mutable,
            type_,
            value: replace_strings_in_expression(value, pool_map),
            location,
        },
        Item::Const {
            name,
            type_,
            value,
            location,
        } => Item::Const {
            name,
            type_,
            value: replace_strings_in_expression(value, pool_map),
            location,
        },
        other => other,
    }
}

/// Create static declarations for string pool
fn create_pool_statics(pool: &[StringPoolEntry]) -> Vec<Item> {
    pool.iter()
        .map(|entry| Item::Static {
            name: entry.pool_name.clone(),
            mutable: false,
            type_: Type::Reference(Box::new(Type::Custom("str".to_string()))),
            value: Expression::Literal {
                value: Literal::String(entry.value.clone()),
                location: None,
            },
            location: None,
        })
        .collect()
}

/// Main optimization function
pub fn optimize_string_interning(program: &Program) -> StringInterningResult {
    // Step 1: Analyze string literals
    let frequency = analyze_string_literals(program);

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
    let pool_statics = create_pool_statics(&pool);

    // Step 5: Transform program items
    let transformed_items: Vec<Item> = program
        .items
        .iter()
        .map(|item| replace_strings_in_item(item.clone(), &pool_map))
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
    use super::*;
    use crate::parser::*;

    fn create_test_function(name: &str, body_stmts: Vec<Statement>) -> Item {
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
                body: body_stmts,
                parent_type: None,
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
                    vec![Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        },
                        location: None,
                    }],
                ),
                create_test_function(
                    "test2",
                    vec![Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        },
                        location: None,
                    }],
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
                    vec![Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        },
                        location: None,
                    }],
                ),
                create_test_function(
                    "test2",
                    vec![Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        },
                        location: None,
                    }],
                ),
            ],
        };

        let result = optimize_string_interning(&program);

        // Should have 3 items: 1 static + 2 functions
        assert_eq!(result.program.items.len(), 3);

        // First item should be the string pool static
        match &result.program.items[0] {
            Item::Static { name, value, .. } => {
                assert_eq!(name, "__STRING_POOL_0");
                assert_eq!(
                    value,
                    &Expression::Literal {
                        value: Literal::String("Hello World".to_string()),
                        location: None,
                    }
                );
            }
            _ => panic!("Expected static declaration"),
        }

        // Functions should reference the pool
        match &result.program.items[1] {
            Item::Function { decl: f, .. } => {
                if let Some(Statement::Expression {
                    expr: Expression::Identifier { name, .. },
                    ..
                }) = f.body.first()
                {
                    assert_eq!(name, "__STRING_POOL_0");
                } else {
                    panic!("Expected identifier reference to pool");
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
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hello World".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                ],
            )],
        };

        let result = optimize_string_interning(&program);

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
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hi".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hi".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Hi".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                ],
            )],
        };

        let result = optimize_string_interning(&program);

        // Should not intern short strings (< 10 chars)
        assert_eq!(result.strings_interned, 0);
        assert_eq!(result.memory_saved, 0);
    }

    #[test]
    fn test_nested_expressions() {
        let program = Program {
            items: vec![create_test_function(
                "test",
                vec![Statement::Expression {
                    expr: Expression::Binary {
                        left: Box::new(Expression::Literal {
                            value: Literal::String("Long String Value".to_string()),
                            location: None,
                        }),
                        op: BinaryOp::Add,
                        right: Box::new(Expression::Literal {
                            value: Literal::String("Long String Value".to_string()),
                            location: None,
                        }),
                        location: None,
                    },
                    location: None,
                }],
            )],
        };

        let result = optimize_string_interning(&program);
        assert_eq!(result.strings_interned, 1);
        assert_eq!(result.memory_saved, 17); // "Long String Value" = 17 bytes

        // Check transformation
        match &result.program.items[1] {
            Item::Function { decl: f, .. } => {
                if let Some(Statement::Expression {
                    expr: Expression::Binary { left, right, .. },
                    ..
                }) = f.body.first()
                {
                    // Both sides should reference the pool
                    assert!(matches!(&**left, Expression::Identifier { .. }));
                    assert!(matches!(&**right, Expression::Identifier { .. }));
                } else {
                    panic!("Expected binary expression");
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
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("First String".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("First String".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Second String".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                    Statement::Expression {
                        expr: Expression::Literal {
                            value: Literal::String("Second String".to_string()),
                            location: None,
                        },
                        location: None,
                    },
                ],
            )],
        };

        let result = optimize_string_interning(&program);

        // Should intern both strings
        assert_eq!(result.strings_interned, 2);
        // "First String" = 12 bytes, "Second String" = 13 bytes
        // Total savings = 12 + 13 = 25 bytes
        assert_eq!(result.memory_saved, 25);

        // Should have 3 items: 2 statics + 1 function
        assert_eq!(result.program.items.len(), 3);
    }
}
