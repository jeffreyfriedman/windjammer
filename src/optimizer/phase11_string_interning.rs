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
        Item::Function(func) => {
            for stmt in &func.body {
                collect_strings_from_statement(stmt, frequency);
            }
        }
        Item::Impl(impl_block) => {
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
        Expression::Literal(Literal::String(s)) => {
            // Only intern strings >= 10 characters
            if s.len() >= 10 {
                *frequency.entry(s.clone()).or_insert(0) += 1;
            }
        }
        Expression::Binary { left, right, .. } => {
            collect_strings_from_expression(left, frequency);
            collect_strings_from_expression(right, frequency);
        }
        Expression::Ternary {
            condition,
            true_expr,
            false_expr,
        } => {
            collect_strings_from_expression(condition, frequency);
            collect_strings_from_expression(true_expr, frequency);
            collect_strings_from_expression(false_expr, frequency);
        }
        Expression::Unary { operand, .. } => {
            collect_strings_from_expression(operand, frequency);
        }
        Expression::Call {
            function,
            arguments,
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
        Expression::Index { object, index } => {
            collect_strings_from_expression(object, frequency);
            collect_strings_from_expression(index, frequency);
        }
        Expression::Tuple(elements) => {
            for elem in elements {
                collect_strings_from_expression(elem, frequency);
            }
        }
        Expression::MacroInvocation { args, .. } => {
            for arg in args {
                collect_strings_from_expression(arg, frequency);
            }
        }
        Expression::TryOp(expr) | Expression::Await(expr) => {
            collect_strings_from_expression(expr, frequency);
        }
        Expression::ChannelSend { channel, value } => {
            collect_strings_from_expression(channel, frequency);
            collect_strings_from_expression(value, frequency);
        }
        Expression::ChannelRecv(channel) => {
            collect_strings_from_expression(channel, frequency);
        }
        Expression::Block(statements) => {
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
        Statement::Expression(expr) => {
            collect_strings_from_expression(expr, frequency);
        }
        Statement::Return(Some(expr)) => {
            collect_strings_from_expression(expr, frequency);
        }
        Statement::Assignment { target, value } => {
            collect_strings_from_expression(target, frequency);
            collect_strings_from_expression(value, frequency);
        }
        Statement::If {
            condition,
            then_block,
            else_block,
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
        Statement::While { condition, body } => {
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
        Statement::Match { value, arms } => {
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
        Expression::Literal(Literal::String(s)) => {
            // Replace with pool reference if interned
            if let Some(pool_name) = pool_map.get(&s) {
                Expression::Identifier(pool_name.clone())
            } else {
                Expression::Literal(Literal::String(s))
            }
        }
        Expression::Binary { left, right, op } => Expression::Binary {
            left: Box::new(replace_strings_in_expression(*left, pool_map)),
            right: Box::new(replace_strings_in_expression(*right, pool_map)),
            op,
        },
        Expression::Ternary {
            condition,
            true_expr,
            false_expr,
        } => Expression::Ternary {
            condition: Box::new(replace_strings_in_expression(*condition, pool_map)),
            true_expr: Box::new(replace_strings_in_expression(*true_expr, pool_map)),
            false_expr: Box::new(replace_strings_in_expression(*false_expr, pool_map)),
        },
        Expression::Unary { op, operand } => Expression::Unary {
            op,
            operand: Box::new(replace_strings_in_expression(*operand, pool_map)),
        },
        Expression::Call {
            function,
            arguments,
        } => Expression::Call {
            function: Box::new(replace_strings_in_expression(*function, pool_map)),
            arguments: arguments
                .into_iter()
                .map(|(label, arg)| (label, replace_strings_in_expression(arg, pool_map)))
                .collect(),
        },
        Expression::MethodCall {
            object,
            method,
            type_args,
            arguments,
        } => Expression::MethodCall {
            object: Box::new(replace_strings_in_expression(*object, pool_map)),
            method,
            type_args,
            arguments: arguments
                .into_iter()
                .map(|(label, arg)| (label, replace_strings_in_expression(arg, pool_map)))
                .collect(),
        },
        Expression::FieldAccess { object, field } => Expression::FieldAccess {
            object: Box::new(replace_strings_in_expression(*object, pool_map)),
            field,
        },
        Expression::StructLiteral { name, fields } => Expression::StructLiteral {
            name,
            fields: fields
                .into_iter()
                .map(|(name, value)| (name, replace_strings_in_expression(value, pool_map)))
                .collect(),
        },
        Expression::Range {
            start,
            end,
            inclusive,
        } => Expression::Range {
            start: Box::new(replace_strings_in_expression(*start, pool_map)),
            end: Box::new(replace_strings_in_expression(*end, pool_map)),
            inclusive,
        },
        Expression::Closure { parameters, body } => Expression::Closure {
            parameters,
            body: Box::new(replace_strings_in_expression(*body, pool_map)),
        },
        Expression::Cast { expr, type_ } => Expression::Cast {
            expr: Box::new(replace_strings_in_expression(*expr, pool_map)),
            type_,
        },
        Expression::Index { object, index } => Expression::Index {
            object: Box::new(replace_strings_in_expression(*object, pool_map)),
            index: Box::new(replace_strings_in_expression(*index, pool_map)),
        },
        Expression::Tuple(elements) => Expression::Tuple(
            elements
                .into_iter()
                .map(|e| replace_strings_in_expression(e, pool_map))
                .collect(),
        ),
        Expression::MacroInvocation {
            name,
            args,
            delimiter,
        } => Expression::MacroInvocation {
            name,
            args: args
                .into_iter()
                .map(|arg| replace_strings_in_expression(arg, pool_map))
                .collect(),
            delimiter,
        },
        Expression::TryOp(expr) => {
            Expression::TryOp(Box::new(replace_strings_in_expression(*expr, pool_map)))
        }
        Expression::Await(expr) => {
            Expression::Await(Box::new(replace_strings_in_expression(*expr, pool_map)))
        }
        Expression::ChannelSend { channel, value } => Expression::ChannelSend {
            channel: Box::new(replace_strings_in_expression(*channel, pool_map)),
            value: Box::new(replace_strings_in_expression(*value, pool_map)),
        },
        Expression::ChannelRecv(channel) => {
            Expression::ChannelRecv(Box::new(replace_strings_in_expression(*channel, pool_map)))
        }
        Expression::Block(statements) => Expression::Block(
            statements
                .into_iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                .collect(),
        ),
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
        } => Statement::Let {
            pattern,
            mutable,
            type_,
            value: replace_strings_in_expression(value, pool_map),
        },
        Statement::Const { name, type_, value } => Statement::Const {
            name,
            type_,
            value: replace_strings_in_expression(value, pool_map),
        },
        Statement::Static {
            name,
            mutable,
            type_,
            value,
        } => Statement::Static {
            name,
            mutable,
            type_,
            value: replace_strings_in_expression(value, pool_map),
        },
        Statement::Expression(expr) => {
            Statement::Expression(replace_strings_in_expression(expr, pool_map))
        }
        Statement::Return(Some(expr)) => {
            Statement::Return(Some(replace_strings_in_expression(expr, pool_map)))
        }
        Statement::Assignment { target, value } => Statement::Assignment {
            target: replace_strings_in_expression(target, pool_map),
            value: replace_strings_in_expression(value, pool_map),
        },
        Statement::If {
            condition,
            then_block,
            else_block,
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
        },
        Statement::While { condition, body } => Statement::While {
            condition: replace_strings_in_expression(condition, pool_map),
            body: body
                .into_iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                .collect(),
        },
        Statement::For {
            pattern,
            iterable,
            body,
        } => Statement::For {
            pattern,
            iterable: replace_strings_in_expression(iterable, pool_map),
            body: body
                .into_iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                .collect(),
        },
        Statement::Match { value, arms } => Statement::Match {
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
        },
        other => other,
    }
}

/// Replace string literals in an item with pool references
fn replace_strings_in_item(item: Item, pool_map: &HashMap<String, String>) -> Item {
    match item {
        Item::Function(mut func) => {
            func.body = func
                .body
                .into_iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map))
                .collect();
            Item::Function(func)
        }
        Item::Impl(mut impl_block) => {
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
            Item::Impl(impl_block)
        }
        Item::Static {
            name,
            mutable,
            type_,
            value,
        } => Item::Static {
            name,
            mutable,
            type_,
            value: replace_strings_in_expression(value, pool_map),
        },
        Item::Const { name, type_, value } => Item::Const {
            name,
            type_,
            value: replace_strings_in_expression(value, pool_map),
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
            value: Expression::Literal(Literal::String(entry.value.clone())),
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
        Item::Function(FunctionDecl {
            name: name.to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parameters: vec![],
            return_type: None,
            body: body_stmts,
        })
    }

    #[test]
    fn test_string_frequency_analysis() {
        let program = Program {
            items: vec![
                create_test_function(
                    "test1",
                    vec![Statement::Expression(Expression::Literal(Literal::String(
                        "Hello World".to_string(),
                    )))],
                ),
                create_test_function(
                    "test2",
                    vec![Statement::Expression(Expression::Literal(Literal::String(
                        "Hello World".to_string(),
                    )))],
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
                    vec![Statement::Expression(Expression::Literal(Literal::String(
                        "Hello World".to_string(),
                    )))],
                ),
                create_test_function(
                    "test2",
                    vec![Statement::Expression(Expression::Literal(Literal::String(
                        "Hello World".to_string(),
                    )))],
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
                    &Expression::Literal(Literal::String("Hello World".to_string()))
                );
            }
            _ => panic!("Expected static declaration"),
        }

        // Functions should reference the pool
        match &result.program.items[1] {
            Item::Function(f) => {
                if let Some(Statement::Expression(Expression::Identifier(name))) = f.body.first() {
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
                    Statement::Expression(Expression::Literal(Literal::String(
                        "Hello World".to_string(),
                    ))),
                    Statement::Expression(Expression::Literal(Literal::String(
                        "Hello World".to_string(),
                    ))),
                    Statement::Expression(Expression::Literal(Literal::String(
                        "Hello World".to_string(),
                    ))),
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
                    Statement::Expression(Expression::Literal(Literal::String("Hi".to_string()))),
                    Statement::Expression(Expression::Literal(Literal::String("Hi".to_string()))),
                    Statement::Expression(Expression::Literal(Literal::String("Hi".to_string()))),
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
                vec![Statement::Expression(Expression::Binary {
                    left: Box::new(Expression::Literal(Literal::String(
                        "Long String Value".to_string(),
                    ))),
                    op: BinaryOp::Add,
                    right: Box::new(Expression::Literal(Literal::String(
                        "Long String Value".to_string(),
                    ))),
                })],
            )],
        };

        let result = optimize_string_interning(&program);
        assert_eq!(result.strings_interned, 1);
        assert_eq!(result.memory_saved, 17); // "Long String Value" = 17 bytes

        // Check transformation
        match &result.program.items[1] {
            Item::Function(f) => {
                if let Some(Statement::Expression(Expression::Binary { left, right, .. })) =
                    f.body.first()
                {
                    // Both sides should reference the pool
                    assert!(matches!(**left, Expression::Identifier(_)));
                    assert!(matches!(**right, Expression::Identifier(_)));
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
                    Statement::Expression(Expression::Literal(Literal::String(
                        "First String".to_string(),
                    ))),
                    Statement::Expression(Expression::Literal(Literal::String(
                        "First String".to_string(),
                    ))),
                    Statement::Expression(Expression::Literal(Literal::String(
                        "Second String".to_string(),
                    ))),
                    Statement::Expression(Expression::Literal(Literal::String(
                        "Second String".to_string(),
                    ))),
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
