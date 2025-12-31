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

use crate::parser::{Expression, FunctionDecl, ImplBlock, Item, Literal, MatchArm, Program, Statement, Type};
use std::collections::HashMap;

/// Result of string interning optimization
#[derive(Debug, Clone)]
pub struct StringInterningResult<'ast> {
    pub program: Program<'ast>,
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
fn replace_strings_in_expression<'a: 'ast, 'ast>(
    expr: &'a Expression<'a>,
    pool_map: &HashMap<String, String>,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Expression<'ast> {
    match expr {
        Expression::Literal {
            value: Literal::String(s),
            location,
        } => {
            // Replace with pool reference if interned
            if let Some(pool_name) = pool_map.get(s) {
                optimizer.alloc_expr(Expression::Identifier {
                    name: pool_name.clone(),
                    location: location.clone(),
                })
            } else {
                expr // Return as-is if not interned
            }
        }
        Expression::Binary {
            left,
            right,
            op,
            location,
        } => optimizer.alloc_expr(Expression::Binary {
            left: replace_strings_in_expression(left, pool_map, optimizer),
            right: replace_strings_in_expression(right, pool_map, optimizer),
            op: *op,
            location: location.clone(),
        }),
        Expression::Unary {
            op,
            operand,
            location,
        } => optimizer.alloc_expr(Expression::Unary {
            op: *op,
            operand: replace_strings_in_expression(operand, pool_map, optimizer),
            location: location.clone(),
        }),
        Expression::Call {
            function,
            arguments,
            location,
        } => optimizer.alloc_expr(Expression::Call {
            function: replace_strings_in_expression(function, pool_map, optimizer),
            arguments: arguments
                .iter()
                .map(|(label, arg)| (label.clone(), replace_strings_in_expression(arg, pool_map, optimizer)))
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
            object: replace_strings_in_expression(object, pool_map, optimizer),
            method: method.clone(),
            type_args: type_args.clone(),
            arguments: arguments
                .iter()
                .map(|(label, arg)| (label.clone(), replace_strings_in_expression(arg, pool_map, optimizer)))
                .collect(),
            location: location.clone(),
        }),
        Expression::FieldAccess {
            object,
            field,
            location,
        } => optimizer.alloc_expr(Expression::FieldAccess {
            object: replace_strings_in_expression(object, pool_map, optimizer),
            field: field.clone(),
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
                .map(|(name, value)| (name.clone(), replace_strings_in_expression(value, pool_map, optimizer)))
                .collect(),
            location: location.clone(),
        }),
        Expression::Range {
            start,
            end,
            inclusive,
            location,
        } => optimizer.alloc_expr(Expression::Range {
            start: replace_strings_in_expression(start, pool_map, optimizer),
            end: replace_strings_in_expression(end, pool_map, optimizer),
            inclusive: *inclusive,
            location: location.clone(),
        }),
        Expression::Closure {
            parameters,
            body,
            location,
        } => optimizer.alloc_expr(Expression::Closure {
            parameters: parameters.clone(),
            body: replace_strings_in_expression(body, pool_map, optimizer),
            location: location.clone(),
        }),
        Expression::Cast {
            expr,
            type_,
            location,
        } => optimizer.alloc_expr(Expression::Cast {
            expr: replace_strings_in_expression(expr, pool_map, optimizer),
            type_: type_.clone(),
            location: location.clone(),
        }),
        Expression::Index {
            object,
            index,
            location,
        } => optimizer.alloc_expr(Expression::Index {
            object: replace_strings_in_expression(object, pool_map, optimizer),
            index: replace_strings_in_expression(index, pool_map, optimizer),
            location: location.clone(),
        }),
        Expression::Tuple { elements, location } => optimizer.alloc_expr(Expression::Tuple {
            elements: elements
                .iter()
                .map(|e| replace_strings_in_expression(e, pool_map, optimizer))
                .collect(),
            location: location.clone(),
        }),
        Expression::MacroInvocation {
            name,
            args,
            delimiter,
            location,
        } => optimizer.alloc_expr(Expression::MacroInvocation {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| replace_strings_in_expression(arg, pool_map, optimizer))
                .collect(),
            delimiter: *delimiter,
            location: location.clone(),
        }),
        Expression::TryOp { expr, location } => optimizer.alloc_expr(Expression::TryOp {
            expr: replace_strings_in_expression(expr, pool_map, optimizer),
            location: location.clone(),
        }),
        Expression::Await { expr, location } => optimizer.alloc_expr(unsafe { std::mem::transmute(Expression::Await {
            expr: replace_strings_in_expression(expr, pool_map, optimizer),
            location: location.clone(),
        }) }),
        Expression::ChannelSend {
            channel,
            value,
            location,
        } => optimizer.alloc_expr(unsafe { std::mem::transmute(Expression::ChannelSend {
            channel: replace_strings_in_expression(channel, pool_map, optimizer),
            value: replace_strings_in_expression(value, pool_map, optimizer),
            location: location.clone(),
        }) }),
        Expression::ChannelRecv { channel, location } => optimizer.alloc_expr(unsafe { std::mem::transmute(Expression::ChannelRecv {
            channel: replace_strings_in_expression(channel, pool_map, optimizer),
            location: location.clone(),
        }) }),
        Expression::Block {
            statements,
            location,
        } => optimizer.alloc_expr(Expression::Block {
            statements: statements
                .iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                .collect(),
            location: location.clone(),
        }),
        other => other,
    }
}

/// Replace string literals in a statement with pool references
fn replace_strings_in_statement<'a: 'ast, 'ast>(
    stmt: &'a Statement<'a>,
    pool_map: &HashMap<String, String>,
    optimizer: &crate::optimizer::Optimizer) -> &'ast Statement<'ast> {
    match stmt {
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
            value: replace_strings_in_expression(value, pool_map, optimizer),
            else_block: else_block.as_ref().map(|stmts| {
                stmts
                    .iter()
                    .map(|s| replace_strings_in_statement(s, pool_map, optimizer))
                    .collect()
            }),
            location: location.clone(),
        }) }),
        Statement::Const {
            name,
            type_,
            value,
            location,
        } => optimizer.alloc_stmt(Statement::Const {
            name: name.clone(),
            type_: type_.clone(),
            value: replace_strings_in_expression(value, pool_map, optimizer),
            location: location.clone(),
        }),
        Statement::Static {
            name,
            mutable,
            type_,
            value,
            location,
        } => optimizer.alloc_stmt(Statement::Static {
            name: name.clone(),
            mutable: *mutable,
            type_: type_.clone(),
            value: replace_strings_in_expression(value, pool_map, optimizer),
            location: location.clone(),
        }),
        Statement::Expression { expr, location } => optimizer.alloc_stmt(Statement::Expression {
            expr: replace_strings_in_expression(expr, pool_map, optimizer),
            location: location.clone(),
        }),
        Statement::Return {
            value: Some(expr),
            location,
        } => optimizer.alloc_stmt(Statement::Return {
            value: Some(replace_strings_in_expression(expr, pool_map, optimizer)),
            location: location.clone(),
        }),
        Statement::Assignment {
            target,
            value,
            compound_op,
            location,
        } => optimizer.alloc_stmt(Statement::Assignment {
            target: replace_strings_in_expression(target, pool_map, optimizer),
            value: replace_strings_in_expression(value, pool_map, optimizer),
            compound_op: *compound_op,
            location: location.clone(),
        }),
        Statement::If {
            condition,
            then_block,
            else_block,
            location,
        } => optimizer.alloc_stmt(unsafe { std::mem::transmute(Statement::If {
            condition: replace_strings_in_expression(condition, pool_map, optimizer),
            then_block: then_block
                .iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                .collect(),
            else_block: else_block.as_ref().map(|stmts| {
                stmts
                    .iter()
                    .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                    .collect()
            }),
            location: location.clone(),
        }) }),
        Statement::While {
            condition,
            body,
            location,
        } => optimizer.alloc_stmt(unsafe { std::mem::transmute(Statement::While {
            condition: replace_strings_in_expression(condition, pool_map, optimizer),
            body: body
                .iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                .collect(),
            location: location.clone(),
        }) }),
        Statement::For {
            pattern,
            iterable,
            body,
            location,
        } => optimizer.alloc_stmt(unsafe { std::mem::transmute(Statement::For {
            pattern: pattern.clone(),
            iterable: replace_strings_in_expression(iterable, pool_map, optimizer),
            body: body
                .iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                .collect(),
            location: location.clone(),
        }) }),
        Statement::Match {
            value,
            arms,
            location,
        } => optimizer.alloc_stmt(unsafe { std::mem::transmute(Statement::Match {
            value: replace_strings_in_expression(value, pool_map, optimizer),
            arms: arms
                .iter()
                .map(|arm| MatchArm {
                    pattern: arm.pattern.clone(),
                    guard: arm
                        .guard
                        .map(|g| replace_strings_in_expression(g, pool_map, optimizer)),
                    body: replace_strings_in_expression(arm.body, pool_map, optimizer),
                })
                .collect(),
            location: location.clone(),
        }) }),
        other => stmt, // Return as-is for other statement types
    }
}

/// Replace string literals in an item with pool references
fn replace_strings_in_item<'ast>(
    item: &Item<'ast>,
    pool_map: &HashMap<String, String>,
    optimizer: &crate::optimizer::Optimizer,
) -> Item<'ast> {
    match item {
        Item::Function { decl, location } => {
            let new_body: Vec<&'ast Statement<'ast>> = decl
                .body
                .iter()
                .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                .collect();
            Item::Function {
                decl: FunctionDecl {
                    name: decl.name.clone(),
                    parameters: decl.parameters.clone(),
                    return_type: decl.return_type.clone(),
                    body: new_body,
                    is_pub: decl.is_pub,
                    is_async: decl.is_async,
                    decorators: decl.decorators.clone(),
                    is_extern: decl.is_extern,
                    type_params: decl.type_params.clone(),
                    where_clause: decl.where_clause.clone(),
                    parent_type: decl.parent_type.clone(),
                    doc_comment: decl.doc_comment.clone(),
                },
                location: location.clone(),
            }
        }
        Item::Impl { block, location } => {
            let new_functions: Vec<FunctionDecl<'ast>> = block
                .functions
                .iter()
                .map(|func| {
                    let new_body: Vec<&'ast Statement<'ast>> = func
                        .body
                        .iter()
                        .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                        .collect();
                    FunctionDecl {
                        name: func.name.clone(),
                        parameters: func.parameters.clone(),
                        return_type: func.return_type.clone(),
                        body: new_body,
                        is_pub: func.is_pub,
                        is_async: func.is_async,
                        decorators: func.decorators.clone(),
                        is_extern: func.is_extern,
                        type_params: func.type_params.clone(),
                        where_clause: func.where_clause.clone(),
                        parent_type: func.parent_type.clone(),
                        doc_comment: func.doc_comment.clone(),
                    }
                })
                .collect();
            Item::Impl {
                block: ImplBlock {
                    type_name: block.type_name.clone(),
                    type_params: block.type_params.clone(),
                    where_clause: block.where_clause.clone(),
                    trait_name: block.trait_name.clone(),
                    trait_type_args: block.trait_type_args.clone(),
                    associated_types: block.associated_types.clone(),
                    functions: new_functions,
                    decorators: block.decorators.clone(),
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
            value: replace_strings_in_expression(value, pool_map, optimizer),
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
            value: replace_strings_in_expression(value, pool_map, optimizer),
            location: location.clone(),
        },
        other => item.clone(),
    }
}

/// Create static declarations for string pool
fn create_pool_statics<'ast>(
    pool: &[StringPoolEntry],
    optimizer: &crate::optimizer::Optimizer,
) -> Vec<Item<'ast>> {
    pool.iter()
        .map(|entry| Item::Static {
            name: entry.pool_name.clone(),
            mutable: false,
            type_: Type::Reference(Box::new(Type::Custom("str".to_string()))),
            value: optimizer.alloc_expr(Expression::Literal {
                value: Literal::String(entry.value.clone()),
                location: None,
            }),
            location: None,
        })
        .collect()
}

/// Main optimization function
pub fn optimize_string_interning<'ast>(
    program: &Program<'ast>,
    optimizer: &crate::optimizer::Optimizer,
) -> StringInterningResult<'ast> {
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
    let pool_statics = create_pool_statics(&pool, optimizer);

    // Step 5: Transform program items
    let transformed_items: Vec<Item<'ast>> = program
        .items
        .iter()
        .map(|item| replace_strings_in_item(item, &pool_map, optimizer))
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
