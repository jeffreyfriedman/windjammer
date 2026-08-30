//! Collection type detection for the Rust code generator.
//!
//! This module provides AST traversal functions to detect usage of collection types
//! (HashMap, HashSet, etc.) in Windjammer programs. Detection is done by walking
//! the AST properly—not by searching debug text, which includes comments and
//! causes false positives.
//!
//! Used to auto-detect when `std::collections::HashMap` and `std::collections::HashSet`
//! imports are needed in the generated Rust code.

use super::CodeGenerator;
use crate::analyzer::FunctionSignature;
use crate::parser::*;

/// Whether a type is a valid `.collect()` turbofish target (Vec, HashSet, etc.).
/// Non-collection return types like `String` or `Option<String>` must not drive collect inference.
pub(crate) fn type_is_collect_turbofish_target(ty: &Type) -> bool {
    match ty {
        Type::Vec(_) => true,
        Type::Parameterized(base, _) => {
            crate::type_classification::is_collect_turbofish_target_base(base)
        }
        _ => false,
    }
}

/// Element type collected into `Vec<T>`, `HashSet<T>`, etc.
pub(crate) fn collect_target_element_type(ty: &Type) -> Option<Type> {
    match ty {
        Type::Vec(inner) => Some(inner.as_ref().clone()),
        Type::Parameterized(base, args)
            if crate::type_classification::is_single_elem_iterable_base(base)
                && args.len() == 1 =>
        {
            Some(args[0].clone())
        }
        _ => None,
    }
}

pub(crate) fn peel_type_reference(ty: &Type) -> &Type {
    match ty {
        Type::Reference(inner) | Type::MutableReference(inner) => peel_type_reference(inner),
        other => other,
    }
}

fn types_equivalent_for_collect(a: &Type, b: &Type) -> bool {
    peel_type_reference(a) == peel_type_reference(b)
}

fn iterator_item_needs_owned_adapter(iter_item: &Type, target_elem: &Type) -> bool {
    matches!(iter_item, Type::Reference(_) | Type::MutableReference(_))
        && !matches!(target_elem, Type::Reference(_) | Type::MutableReference(_))
        && types_equivalent_for_collect(iter_item, target_elem)
}

/// Borrowed text iterator items (`&str`, `&String`) collected into `Vec<string>` need `Vec<_>`,
/// not `collect::<Vec<String>>()`, when the consumer can coerce per element (e.g. a for-loop).
fn iterator_collect_should_use_inferred_vec(iter_item: &Type, target_elem: &Type) -> bool {
    matches!(iter_item, Type::Reference(_) | Type::MutableReference(_))
        && crate::codegen::rust::types::is_windjammer_text_type(target_elem)
        && crate::codegen::rust::types::is_windjammer_text_type(peel_type_reference(iter_item))
        && !types_equivalent_for_collect(iter_item, target_elem)
}

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
            Item::ExternLet { type_, .. } => Self::type_references_name(type_, type_name),
            Item::Mod { items, .. } => items
                .iter()
                .any(|i| Self::item_references_collection(i, type_name)),
            Item::Use { .. } | Item::BoundAlias { .. } => false,
            Item::TypeAlias { target, .. } => Self::type_references_name(target, type_name),
            Item::Macro { .. } => false,
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
            Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. }
            | Expression::AsyncCall { expr, .. }
            | Expression::SpawnCall { expr, .. } => {
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

    /// Infer `Iterator::Item` at a `.collect()` receiver from registry metadata and receiver types.
    pub(in crate::codegen::rust) fn infer_iterator_item_type_at_expr(
        &self,
        expr: &Expression,
    ) -> Option<Type> {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                if method == "into_iter" {
                    let recv_ty = self.infer_expression_type(object)?;
                    return match recv_ty {
                        Type::Reference(inner) | Type::MutableReference(inner) => {
                            Self::extract_iterator_element_type(&inner)
                                .map(|elem| Type::Reference(Box::new(elem)))
                        }
                        other => Self::extract_iterator_element_type(&other),
                    };
                }

                if let Some(item) = self
                    .lookup_method_function_signature_for_iterator_item(object, method)
                    .and_then(|sig| {
                        crate::codegen::rust::stdlib_method_traits::iterator_item_type_from_sig(
                            &sig,
                        )
                    })
                {
                    return Some(item);
                }

                if crate::codegen::rust::stdlib_method_traits::method_returns_iterator_qualified(
                    method,
                    self.mc_infer_method_receiver_type_name(object)
                        .or_else(|| self.infer_type_name(object))
                        .as_deref(),
                    &self.signature_registry,
                ) || {
                    let recv = self
                        .mc_infer_method_receiver_type_name(object)
                        .or_else(|| self.infer_type_name(object));
                    crate::codegen::rust::stdlib_method_traits::method_is_closure_taking_qualified(
                        method,
                        recv.as_deref(),
                        &self.signature_registry,
                    )
                } {
                    return self.infer_iterator_item_type_at_expr(object);
                }

                None
            }
            _ => None,
        }
    }

    fn lookup_method_function_signature_for_iterator_item(
        &self,
        object: &Expression,
        method: &str,
    ) -> Option<FunctionSignature> {
        let receiver = self
            .mc_infer_method_receiver_type_name(object)
            .or_else(|| self.infer_type_name(object))?;
        let qualified = format!("{receiver}::{method}");
        self.get_signature_with_global(&qualified)
            .cloned()
            .or_else(|| self.signature_registry.get_signature(&qualified).cloned())
            .or_else(|| {
                let base = receiver.split('<').next().unwrap_or(&receiver);
                if base == receiver {
                    None
                } else {
                    let q = format!("{base}::{method}");
                    self.get_signature_with_global(&q)
                        .cloned()
                        .or_else(|| self.signature_registry.get_signature(&q).cloned())
                }
            })
    }

    /// Type-directed `.collect()` lowering: adapter suffix (`.copied()`, etc.) and turbofish.
    pub(in crate::codegen::rust) fn compute_collect_lowering(
        &self,
        collect_receiver: &Expression,
    ) -> (String, String) {
        let iter_item = self.infer_iterator_item_type_at_expr(collect_receiver);
        let target_ty = self.collect_target_type.as_ref().or_else(|| {
            self.current_function_return_type
                .as_ref()
                .filter(|t| type_is_collect_turbofish_target(t))
        });
        let target_elem = target_ty
            .as_ref()
            .and_then(|t| collect_target_element_type(t));

        let (adapter, turbofish) = match (iter_item.as_ref(), target_elem.as_ref()) {
            (Some(iter), Some(target))
                if iterator_collect_should_use_inferred_vec(iter, target) =>
            {
                (String::new(), "::<Vec<_>>".to_string())
            }
            (Some(iter), Some(target)) if iterator_item_needs_owned_adapter(iter, target) => {
                let adapter = if crate::codegen::rust::type_analysis_pure::is_copy_type(
                    peel_type_reference(iter),
                ) {
                    ".copied()".to_string()
                } else if crate::codegen::rust::types::is_windjammer_text_type(peel_type_reference(
                    iter,
                )) {
                    ".map(|s| s.to_string())".to_string()
                } else {
                    ".cloned()".to_string()
                };
                let turbofish = target_ty
                    .map(|t| format!("::<{}>", self.type_to_rust(t)))
                    .unwrap_or_else(|| "::<Vec<_>>".to_string());
                (adapter, turbofish)
            }
            (Some(iter), Some(target)) if types_equivalent_for_collect(iter, target) => {
                let turbofish = target_ty
                    .map(|t| format!("::<{}>", self.type_to_rust(t)))
                    .unwrap_or_else(|| "::<Vec<_>>".to_string());
                (String::new(), turbofish)
            }
            _ => (
                String::new(),
                target_ty
                    .map(|t| format!("::<{}>", self.type_to_rust(t)))
                    .unwrap_or_else(|| "::<Vec<_>>".to_string()),
            ),
        };

        (adapter, turbofish)
    }

    /// When `find`/`find_map`-style adapters yield `Option<&T>` from a borrowed
    /// iterator but the enclosing expression needs `Option<T>`, append `.cloned()`.
    /// Driven by inferred iterator item type vs owned option payload — not method name.
    pub(in crate::codegen::rust) fn find_needs_cloned_for_owned_return(
        &self,
        find_receiver_chain: &Expression,
    ) -> bool {
        // Only iterator chains (…into_iter()/iter()/filter()/map()) — never plain
        // string/text receivers (`s.find(pattern)` returns `Option<usize>`).
        if !Self::expr_is_iterator_adapter_chain(find_receiver_chain) {
            return false;
        }
        let Some(Type::Option(inner)) = self.current_function_return_type.as_ref() else {
            return false;
        };
        if matches!(
            inner.as_ref(),
            Type::Reference(_) | Type::MutableReference(_)
        ) {
            return false;
        }
        let Some(iter_item) = self.infer_iterator_item_type_at_expr(find_receiver_chain) else {
            return false;
        };
        matches!(iter_item, Type::Reference(_) | Type::MutableReference(_))
            && types_equivalent_for_collect(&iter_item, inner)
    }

    fn expr_is_iterator_adapter_chain(expr: &Expression) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let has_closure_arg = arguments
                    .iter()
                    .any(|(_, a)| matches!(a, Expression::Closure { .. }));
                // Language-level iterator producers / adapters (not ownership heuristics).
                let is_iter_producer = matches!(
                    method.as_str(),
                    "into_iter" | "iter" | "iter_mut" | "copied" | "cloned"
                );
                has_closure_arg || is_iter_producer || Self::expr_is_iterator_adapter_chain(object)
            }
            _ => false,
        }
    }

    /// Element type when `into_iter()` is invoked on a borrowed collection parameter/local.
    pub(in crate::codegen::rust) fn infer_borrowed_collection_element_type(
        &self,
        object: &Expression,
    ) -> Option<Type> {
        let recv_ty = self.infer_expression_type(object)?;
        let is_borrowed = matches!(recv_ty, Type::Reference(_) | Type::MutableReference(_))
            || matches!(
                object,
                Expression::Identifier { name, .. }
                    if self.inferred_borrowed_params.contains(name)
                        || self.inferred_mut_borrowed_params.contains(name)
            );
        if !is_borrowed {
            return None;
        }
        let container = match &recv_ty {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        Self::extract_iterator_element_type(container)
    }

    /// Discover `strings::`, `path::`, … references so generated Rust gets runtime `use` lines.
    ///
    /// `imported_runtime_std_modules` is the set of runtime std modules already brought into
    /// scope via `use std::foo` in this file. Bare identifiers (e.g. a local `map: HashMap`)
    /// are only treated as module receivers when listed there — never by homonym with scanned
    /// std module names.
    pub(super) fn collect_runtime_std_module_refs_from_program(
        program: &Program,
        imported_runtime_std_modules: &std::collections::HashSet<String>,
    ) -> std::collections::HashSet<String> {
        let mut mods = std::collections::HashSet::new();
        for item in &program.items {
            Self::item_collect_runtime_std_modules(item, imported_runtime_std_modules, &mut mods);
        }
        mods
    }

    fn item_collect_runtime_std_modules(
        item: &Item,
        imported_runtime_std_modules: &std::collections::HashSet<String>,
        mods: &mut std::collections::HashSet<String>,
    ) {
        match item {
            Item::Function { decl, .. } => {
                for stmt in &decl.body {
                    Self::stmt_collect_runtime_std_modules(
                        stmt,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Item::Impl { block, .. } => {
                for method in &block.functions {
                    for stmt in &method.body {
                        Self::stmt_collect_runtime_std_modules(
                            stmt,
                            imported_runtime_std_modules,
                            mods,
                        );
                    }
                }
            }
            Item::Mod { items, .. } => {
                for child in items {
                    Self::item_collect_runtime_std_modules(
                        child,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Item::Const { value, .. } | Item::Static { value, .. } => {
                Self::expr_collect_runtime_std_modules(
                    value,
                    imported_runtime_std_modules,
                    mods,
                );
            }
            _ => {}
        }
    }

    fn stmt_collect_runtime_std_modules(
        stmt: &Statement,
        imported_runtime_std_modules: &std::collections::HashSet<String>,
        mods: &mut std::collections::HashSet<String>,
    ) {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => Self::expr_collect_runtime_std_modules(
                expr,
                imported_runtime_std_modules,
                mods,
            ),
            Statement::Let { value, .. }
            | Statement::Const { value, .. }
            | Statement::Static { value, .. }
            | Statement::Assignment { value, .. } => {
                Self::expr_collect_runtime_std_modules(
                    value,
                    imported_runtime_std_modules,
                    mods,
                );
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::expr_collect_runtime_std_modules(
                    condition,
                    imported_runtime_std_modules,
                    mods,
                );
                for s in then_block {
                    Self::stmt_collect_runtime_std_modules(
                        s,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
                if let Some(e) = else_block {
                    for s in e {
                        Self::stmt_collect_runtime_std_modules(
                            s,
                            imported_runtime_std_modules,
                            mods,
                        );
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                Self::expr_collect_runtime_std_modules(
                    condition,
                    imported_runtime_std_modules,
                    mods,
                );
                for s in body {
                    Self::stmt_collect_runtime_std_modules(
                        s,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Statement::For { iterable, body, .. } => {
                Self::expr_collect_runtime_std_modules(
                    iterable,
                    imported_runtime_std_modules,
                    mods,
                );
                for s in body {
                    Self::stmt_collect_runtime_std_modules(
                        s,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                for s in body {
                    Self::stmt_collect_runtime_std_modules(
                        s,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Statement::Match { value, arms, .. } => {
                Self::expr_collect_runtime_std_modules(
                    value,
                    imported_runtime_std_modules,
                    mods,
                );
                for arm in arms {
                    if let Some(g) = arm.guard {
                        Self::expr_collect_runtime_std_modules(
                            g,
                            imported_runtime_std_modules,
                            mods,
                        );
                    }
                    Self::expr_collect_runtime_std_modules(
                        &arm.body,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Statement::Defer { statement, .. } => {
                Self::stmt_collect_runtime_std_modules(
                    statement,
                    imported_runtime_std_modules,
                    mods,
                );
            }
            _ => {}
        }
    }

    fn expr_collect_runtime_std_modules(
        expr: &Expression,
        imported_runtime_std_modules: &std::collections::HashSet<String>,
        mods: &mut std::collections::HashSet<String>,
    ) {
        if let Some(module) = Self::expr_runtime_std_module_segment(
            expr,
            imported_runtime_std_modules,
        ) {
            mods.insert(module);
        }
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                Self::expr_collect_runtime_std_modules(
                    function,
                    imported_runtime_std_modules,
                    mods,
                );
                for (_, arg) in arguments {
                    Self::expr_collect_runtime_std_modules(
                        arg,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Expression::MethodCall {
                object,
                arguments,
                ..
            } => {
                Self::expr_collect_runtime_std_modules(
                    object,
                    imported_runtime_std_modules,
                    mods,
                );
                for (_, arg) in arguments {
                    Self::expr_collect_runtime_std_modules(
                        arg,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Expression::FieldAccess { object, .. } => {
                Self::expr_collect_runtime_std_modules(
                    object,
                    imported_runtime_std_modules,
                    mods,
                );
            }
            Expression::Binary { left, right, .. } => {
                Self::expr_collect_runtime_std_modules(
                    left,
                    imported_runtime_std_modules,
                    mods,
                );
                Self::expr_collect_runtime_std_modules(
                    right,
                    imported_runtime_std_modules,
                    mods,
                );
            }
            Expression::Unary { operand, .. } => {
                Self::expr_collect_runtime_std_modules(
                    operand,
                    imported_runtime_std_modules,
                    mods,
                );
            }
            Expression::Index { object, index, .. } => {
                Self::expr_collect_runtime_std_modules(
                    object,
                    imported_runtime_std_modules,
                    mods,
                );
                Self::expr_collect_runtime_std_modules(
                    index,
                    imported_runtime_std_modules,
                    mods,
                );
            }
            Expression::Block { statements, .. } => {
                for s in statements {
                    Self::stmt_collect_runtime_std_modules(
                        s,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Expression::Array { elements, .. } | Expression::Tuple { elements, .. } => {
                for e in elements {
                    Self::expr_collect_runtime_std_modules(
                        e,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, v) in fields {
                    Self::expr_collect_runtime_std_modules(
                        v,
                        imported_runtime_std_modules,
                        mods,
                    );
                }
            }
            Expression::Cast { expr, .. }
            | Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. } => Self::expr_collect_runtime_std_modules(
                expr,
                imported_runtime_std_modules,
                mods,
            ),
            _ => {}
        }
    }

    fn expr_runtime_std_module_segment(
        expr: &Expression,
        imported_runtime_std_modules: &std::collections::HashSet<String>,
    ) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => {
                if let Some((module, _)) = name.split_once("::") {
                    if crate::codegen::rust::stdlib_method_traits::is_runtime_std_module(module) {
                        return Some(module.to_string());
                    }
                }
                None
            }
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if imported_runtime_std_modules.contains(name) {
                        return Some(name.clone());
                    }
                }
                Self::expr_runtime_std_module_segment(object, imported_runtime_std_modules)
            }
            Expression::MethodCall { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if imported_runtime_std_modules.contains(name) {
                        return Some(name.clone());
                    }
                }
                Self::expr_runtime_std_module_segment(object, imported_runtime_std_modules)
            }
            Expression::Call { function, .. } => {
                Self::expr_runtime_std_module_segment(function, imported_runtime_std_modules)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod runtime_std_collect_tests {
    use super::CodeGenerator;
    use crate::codegen::rust::stdlib_method_traits::is_runtime_std_module;
    use crate::compiler::parse_wj_source;
    use std::path::Path;

    #[test]
    fn strings_is_runtime_std_module() {
        assert!(
            is_runtime_std_module("strings"),
            "stdlib scanner must register strings as runtime std module"
        );
    }

    #[test]
    fn method_call_strings_len_collects_import() {
        let source = r#"
pub struct AssetLoader {
    loads: int,
}

impl AssetLoader {
    pub fn load(self, path: string) -> int {
        self.loads = self.loads + 1
        strings::len(path)
    }
}
"#;
        let (_parser, program) =
            parse_wj_source(Path::new("assets/loader.wj"), source).expect("parse");
        let mods = CodeGenerator::collect_runtime_std_module_refs_from_program(
            &program,
            &std::collections::HashSet::new(),
        );
        assert!(
            mods.contains("strings"),
            "strings::len qualified Identifier call must collect strings for implicit import, got {mods:?}"
        );
    }

    #[test]
    fn local_named_map_does_not_collect_runtime_std_map_import() {
        let source = r#"
use std::collections::HashMap

pub fn bfs_distance(map: HashMap<i64, i64>, vertex: i64) -> i64 {
    if map.contains_key(vertex) {
        return 0
    }
    -1
}
"#;
        let (_parser, program) =
            parse_wj_source(Path::new("graph/hashmap_i64_bfs_keys.wj"), source).expect("parse");
        let mods = CodeGenerator::collect_runtime_std_module_refs_from_program(
            &program,
            &std::collections::HashSet::new(),
        );
        assert!(
            !mods.contains("map"),
            "local HashMap param named map must not trigger windjammer_runtime::map import, got {mods:?}"
        );
    }
}
