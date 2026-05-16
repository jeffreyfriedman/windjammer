//! AST replacement of string literals with pool identifiers.

#![allow(clippy::transmute_undefined_repr)]

use crate::parser::{Expression, FunctionDecl, ImplBlock, Item, Literal, MatchArm, Statement};
use std::collections::HashMap;

/// Replace string literals in an expression with pool references
pub(super) fn replace_strings_in_expression<'a: 'ast, 'ast>(
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
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Binary {
                left: replace_strings_in_expression(left, pool_map, optimizer),
                right: replace_strings_in_expression(right, pool_map, optimizer),
                op: *op,
                location: location.clone(),
            })
        }),
        Expression::Unary {
            op,
            operand,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Unary {
                op: *op,
                operand: replace_strings_in_expression(operand, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Call {
            function,
            arguments,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Call {
                function: replace_strings_in_expression(function, pool_map, optimizer),
                arguments: arguments
                    .iter()
                    .map(|(label, arg)| {
                        (
                            label.clone(),
                            replace_strings_in_expression(arg, pool_map, optimizer),
                        )
                    })
                    .collect(),
                location: location.clone(),
            })
        }),
        Expression::MethodCall {
            object,
            method,
            type_args,
            arguments,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::MethodCall {
                object: replace_strings_in_expression(object, pool_map, optimizer),
                method: method.clone(),
                type_args: type_args.clone(),
                arguments: arguments
                    .iter()
                    .map(|(label, arg)| {
                        (
                            label.clone(),
                            replace_strings_in_expression(arg, pool_map, optimizer),
                        )
                    })
                    .collect(),
                location: location.clone(),
            })
        }),
        Expression::FieldAccess {
            object,
            field,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::FieldAccess {
                object: replace_strings_in_expression(object, pool_map, optimizer),
                field: field.clone(),
                location: location.clone(),
            })
        }),
        Expression::StructLiteral {
            name,
            fields,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::StructLiteral {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.clone(),
                            replace_strings_in_expression(value, pool_map, optimizer),
                        )
                    })
                    .collect(),
                location: location.clone(),
            })
        }),
        Expression::Range {
            start,
            end,
            inclusive,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Range {
                start: replace_strings_in_expression(start, pool_map, optimizer),
                end: replace_strings_in_expression(end, pool_map, optimizer),
                inclusive: *inclusive,
                location: location.clone(),
            })
        }),
        Expression::Closure {
            parameters,
            body,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Closure {
                parameters: parameters.clone(),
                body: replace_strings_in_expression(body, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Cast {
            expr,
            type_,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Cast {
                expr: replace_strings_in_expression(expr, pool_map, optimizer),
                type_: type_.clone(),
                location: location.clone(),
            })
        }),
        Expression::Index {
            object,
            index,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Index {
                object: replace_strings_in_expression(object, pool_map, optimizer),
                index: replace_strings_in_expression(index, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Tuple { elements, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Tuple {
                elements: elements
                    .iter()
                    .map(|e| replace_strings_in_expression(e, pool_map, optimizer))
                    .collect(),
                location: location.clone(),
            })
        }),
        Expression::MacroInvocation {
            name,
            args,
            delimiter,
            is_repeat,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::MacroInvocation {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|arg| replace_strings_in_expression(arg, pool_map, optimizer))
                    .collect(),
                delimiter: *delimiter,
                is_repeat: *is_repeat,
                location: location.clone(),
            })
        }),
        Expression::TryOp { expr, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::TryOp {
                expr: replace_strings_in_expression(expr, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Await { expr, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Await {
                expr: replace_strings_in_expression(expr, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Expression::ChannelSend {
            channel,
            value,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::ChannelSend {
                channel: replace_strings_in_expression(channel, pool_map, optimizer),
                value: replace_strings_in_expression(value, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Expression::ChannelRecv { channel, location } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::ChannelRecv {
                channel: replace_strings_in_expression(channel, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Expression::Block {
            statements,
            is_unsafe,
            location,
        } => optimizer.alloc_expr(unsafe {
            std::mem::transmute::<Expression<'_>, Expression<'_>>(Expression::Block {
                statements: statements
                    .iter()
                    .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                    .collect(),
                is_unsafe: *is_unsafe,
                location: location.clone(),
            })
        }),
        other => other,
    }
}

/// Replace string literals in a statement with pool references
pub(super) fn replace_strings_in_statement<'a: 'ast, 'ast>(
    stmt: &'a Statement<'a>,
    pool_map: &HashMap<String, String>,
    optimizer: &crate::optimizer::Optimizer,
) -> &'ast Statement<'ast> {
    match stmt {
        Statement::Let {
            pattern,
            mutable,
            type_,
            value,
            else_block,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Let {
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
            })
        }),
        Statement::Const {
            name,
            type_,
            value,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Const {
                name: name.clone(),
                type_: type_.clone(),
                value: replace_strings_in_expression(value, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Statement::Static {
            name,
            mutable,
            type_,
            value,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Static {
                name: name.clone(),
                mutable: *mutable,
                type_: type_.clone(),
                value: replace_strings_in_expression(value, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Statement::Expression { expr, location } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Expression {
                expr: replace_strings_in_expression(expr, pool_map, optimizer),
                location: location.clone(),
            })
        }),
        Statement::Return {
            value: Some(expr),
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Return {
                value: Some(replace_strings_in_expression(expr, pool_map, optimizer)),
                location: location.clone(),
            })
        }),
        Statement::Assignment {
            target,
            value,
            compound_op,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Assignment {
                target: replace_strings_in_expression(target, pool_map, optimizer),
                value: replace_strings_in_expression(value, pool_map, optimizer),
                compound_op: *compound_op,
                location: location.clone(),
            })
        }),
        Statement::If {
            condition,
            then_block,
            else_block,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::If {
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
            })
        }),
        Statement::While {
            condition,
            body,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::While {
                condition: replace_strings_in_expression(condition, pool_map, optimizer),
                body: body
                    .iter()
                    .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                    .collect(),
                location: location.clone(),
            })
        }),
        Statement::For {
            pattern,
            iterable,
            body,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::For {
                pattern: pattern.clone(),
                iterable: replace_strings_in_expression(iterable, pool_map, optimizer),
                body: body
                    .iter()
                    .map(|stmt| replace_strings_in_statement(stmt, pool_map, optimizer))
                    .collect(),
                location: location.clone(),
            })
        }),
        Statement::Match {
            value,
            arms,
            location,
        } => optimizer.alloc_stmt(unsafe {
            std::mem::transmute::<Statement<'_>, Statement<'_>>(Statement::Match {
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
            })
        }),
        _ => stmt, // Return as-is for other statement types
    }
}

/// Replace string literals in an item with pool references
pub(super) fn replace_strings_in_item<'ast>(
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
                    return_decorators: decl.return_decorators.clone(),
                    body: new_body,
                    is_pub: decl.is_pub,
                    is_async: decl.is_async,
                    decorators: decl.decorators.clone(),
                    is_extern: decl.is_extern,
                    type_params: decl.type_params.clone(),
                    where_clause: decl.where_clause.clone(),
                    parent_type: decl.parent_type.clone(),
                    impl_trait: decl.impl_trait.clone(),
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
                        return_decorators: func.return_decorators.clone(),
                        body: new_body,
                        is_pub: func.is_pub,
                        is_async: func.is_async,
                        decorators: func.decorators.clone(),
                        is_extern: func.is_extern,
                        type_params: func.type_params.clone(),
                        where_clause: func.where_clause.clone(),
                        parent_type: func.parent_type.clone(),
                        impl_trait: func.impl_trait.clone(),
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
                    is_extern: block.is_extern,
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
            is_pub,
            location,
        } => Item::Const {
            name: name.clone(),
            type_: type_.clone(),
            value: replace_strings_in_expression(value, pool_map, optimizer),
            is_pub: *is_pub,
            location: location.clone(),
        },
        _ => item.clone(),
    }
}
