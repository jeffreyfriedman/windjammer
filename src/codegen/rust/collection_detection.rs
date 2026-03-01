//! Collection type detection for the Rust code generator.
//!
//! This module provides AST traversal functions to detect usage of collection types
//! (HashMap, HashSet, etc.) in Windjammer programs. Detection is done by walking
//! the AST properly—not by searching debug text, which includes comments and
//! causes false positives.
//!
//! Used to auto-detect when `std::collections::HashMap` and `std::collections::HashSet`
//! imports are needed in the generated Rust code.

use crate::parser::*;
use super::CodeGenerator;

impl CodeGenerator<'_> {
    /// Check if a program references a collection type (HashMap or HashSet)
    /// by walking the AST properly -- not by searching debug text which
    /// includes comments and causes false positives.
    pub(super) fn program_references_collection(program: &Program, type_name: &str) -> bool {
        for item in &program.items {
            if Self::item_references_collection(item, type_name) {
                return true;
            }
        }
        false
    }

    /// Check if an AST item references the given collection type name
    fn item_references_collection(item: &Item, type_name: &str) -> bool {
        match item {
            Item::Struct { decl, .. } => decl
                .fields
                .iter()
                .any(|f| Self::type_references_name(&f.field_type, type_name)),
            Item::Function { decl, .. } => {
                Self::function_decl_references_collection(decl, type_name)
            }
            Item::Enum { decl, .. } => decl.variants.iter().any(|v| match &v.data {
                EnumVariantData::Tuple(types) => types
                    .iter()
                    .any(|t| Self::type_references_name(t, type_name)),
                EnumVariantData::Struct(fields) => fields
                    .iter()
                    .any(|(_, t)| Self::type_references_name(t, type_name)),
                EnumVariantData::Unit => false,
            }),
            Item::Trait { decl, .. } => decl.methods.iter().any(|m| {
                // TraitMethod has parameters + return_type but different structure than FunctionDecl
                m.parameters
                    .iter()
                    .any(|p| Self::type_references_name(&p.type_, type_name))
                    || m.return_type
                        .as_ref()
                        .is_some_and(|rt| Self::type_references_name(rt, type_name))
                    || m.body.as_ref().is_some_and(|stmts| {
                        stmts
                            .iter()
                            .any(|s| Self::stmt_references_collection(s, type_name))
                    })
            }),
            Item::Impl { block, .. } => block
                .functions
                .iter()
                .any(|m| Self::function_decl_references_collection(m, type_name)),
            Item::Const { type_, value, .. } | Item::Static { type_, value, .. } => {
                Self::type_references_name(type_, type_name)
                    || Self::expr_references_collection(value, type_name)
            }
            Item::Mod { items, .. } => items
                .iter()
                .any(|i| Self::item_references_collection(i, type_name)),
            Item::Use { .. } | Item::BoundAlias { .. } => false,
            Item::TypeAlias { target, .. } => Self::type_references_name(target, type_name),
        }
    }

    /// Check if a function declaration references the collection type
    fn function_decl_references_collection(decl: &FunctionDecl, type_name: &str) -> bool {
        // Check parameter types
        if decl
            .parameters
            .iter()
            .any(|p| Self::type_references_name(&p.type_, type_name))
        {
            return true;
        }
        // Check return type
        if let Some(ref rt) = decl.return_type {
            if Self::type_references_name(rt, type_name) {
                return true;
            }
        }
        // Check body statements for type usage in expressions
        decl.body
            .iter()
            .any(|s| Self::stmt_references_collection(s, type_name))
    }

    /// Recursively check if a Type references the given name
    fn type_references_name(ty: &Type, name: &str) -> bool {
        match ty {
            Type::Custom(n) => n == name,
            Type::Parameterized(n, args) => {
                n == name || args.iter().any(|a| Self::type_references_name(a, name))
            }
            Type::Vec(inner)
            | Type::Option(inner)
            | Type::Reference(inner)
            | Type::MutableReference(inner)
            | Type::Array(inner, _) => Self::type_references_name(inner, name),
            Type::Result(a, b) => {
                Self::type_references_name(a, name) || Self::type_references_name(b, name)
            }
            Type::Tuple(types) => types.iter().any(|t| Self::type_references_name(t, name)),
            Type::FunctionPointer {
                params,
                return_type,
            } => {
                params.iter().any(|p| Self::type_references_name(p, name))
                    || return_type
                        .as_ref()
                        .is_some_and(|rt| Self::type_references_name(rt, name))
            }
            _ => false, // Primitives, Generic, Associated, TraitObject, Infer
        }
    }

    /// Check if a statement references the collection type (in let types, expressions, etc.)
    fn stmt_references_collection(stmt: &Statement, type_name: &str) -> bool {
        match stmt {
            Statement::Let { type_, value, .. } => {
                type_
                    .as_ref()
                    .is_some_and(|t| Self::type_references_name(t, type_name))
                    || Self::expr_references_collection(value, type_name)
            }
            Statement::Const { type_, value, .. } | Statement::Static { type_, value, .. } => {
                Self::type_references_name(type_, type_name)
                    || Self::expr_references_collection(value, type_name)
            }
            Statement::Assignment { target, value, .. } => {
                Self::expr_references_collection(target, type_name)
                    || Self::expr_references_collection(value, type_name)
            }
            Statement::Return { value, .. } => value
                .as_ref()
                .is_some_and(|v| Self::expr_references_collection(v, type_name)),
            Statement::Expression { expr, .. } => Self::expr_references_collection(expr, type_name),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::expr_references_collection(condition, type_name)
                    || then_block
                        .iter()
                        .any(|s| Self::stmt_references_collection(s, type_name))
                    || else_block.as_ref().is_some_and(|eb| {
                        eb.iter()
                            .any(|s| Self::stmt_references_collection(s, type_name))
                    })
            }
            Statement::Match { value, arms, .. } => {
                Self::expr_references_collection(value, type_name)
                    || arms.iter().any(|arm| {
                        Self::expr_references_collection(arm.body, type_name)
                            || arm
                                .guard
                                .is_some_and(|g| Self::expr_references_collection(g, type_name))
                    })
            }
            Statement::For { iterable, body, .. } => {
                Self::expr_references_collection(iterable, type_name)
                    || body
                        .iter()
                        .any(|s| Self::stmt_references_collection(s, type_name))
            }
            Statement::While {
                condition, body, ..
            } => {
                Self::expr_references_collection(condition, type_name)
                    || body
                        .iter()
                        .any(|s| Self::stmt_references_collection(s, type_name))
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => body
                .iter()
                .any(|s| Self::stmt_references_collection(s, type_name)),
            Statement::Defer { statement, .. } => {
                Self::stmt_references_collection(statement, type_name)
            }
            Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => false,
        }
    }

    /// Check if an expression references the collection type (identifiers, struct literals, etc.)
    fn expr_references_collection(expr: &Expression, type_name: &str) -> bool {
        match expr {
            // HashMap::new() or HashSet::new() - the identifier itself
            Expression::Identifier { name, .. } => name == type_name,
            // Struct literal: HashMap { ... }
            Expression::StructLiteral { name, fields, .. } => {
                name == type_name
                    || fields
                        .iter()
                        .any(|(_, e)| Self::expr_references_collection(e, type_name))
            }
            // Function/method calls
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                Self::expr_references_collection(function, type_name)
                    || arguments
                        .iter()
                        .any(|(_, e)| Self::expr_references_collection(e, type_name))
            }
            Expression::MethodCall {
                object,
                type_args,
                arguments,
                ..
            } => {
                Self::expr_references_collection(object, type_name)
                    || type_args.as_ref().is_some_and(|args| {
                        args.iter()
                            .any(|t| Self::type_references_name(t, type_name))
                    })
                    || arguments
                        .iter()
                        .any(|(_, e)| Self::expr_references_collection(e, type_name))
            }
            Expression::FieldAccess { object, .. } => {
                Self::expr_references_collection(object, type_name)
            }
            Expression::Binary { left, right, .. } => {
                Self::expr_references_collection(left, type_name)
                    || Self::expr_references_collection(right, type_name)
            }
            Expression::Unary { operand, .. } => {
                Self::expr_references_collection(operand, type_name)
            }
            Expression::Index { object, index, .. } => {
                Self::expr_references_collection(object, type_name)
                    || Self::expr_references_collection(index, type_name)
            }
            Expression::Cast { expr, type_, .. } => {
                Self::expr_references_collection(expr, type_name)
                    || Self::type_references_name(type_, type_name)
            }
            Expression::Array { elements, .. } | Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|e| Self::expr_references_collection(e, type_name)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                Self::expr_references_collection(k, type_name)
                    || Self::expr_references_collection(v, type_name)
            }),
            Expression::Range { start, end, .. } => {
                Self::expr_references_collection(start, type_name)
                    || Self::expr_references_collection(end, type_name)
            }
            Expression::Closure { body, .. } => Self::expr_references_collection(body, type_name),
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| Self::stmt_references_collection(s, type_name)),
            Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
                Self::expr_references_collection(expr, type_name)
            }
            Expression::ChannelSend { channel, value, .. } => {
                Self::expr_references_collection(channel, type_name)
                    || Self::expr_references_collection(value, type_name)
            }
            Expression::ChannelRecv { channel, .. } => {
                Self::expr_references_collection(channel, type_name)
            }
            Expression::MacroInvocation { args, .. } => args
                .iter()
                .any(|e| Self::expr_references_collection(e, type_name)),
            Expression::Literal { .. } => false,
        }
    }
}
