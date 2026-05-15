//! Inferring `&mut self` upgrades for dispatched `for` loops over `self` fields.
use std::collections::HashSet;

use crate::parser::*;

use super::Analyzer;
impl<'ast> Analyzer<'ast> {
    /// `for x in self.field` where the body calls `&mut self` methods on `x` requires `&mut self`
    /// on the outer function (codegen will emit `for x in &mut self.field`).
    pub(super) fn maybe_upgrade_self_for_dispatch_for_loops(
        &self,
        analyzed: &mut super::AnalyzedFunction<'ast>,
        func: &FunctionDecl<'ast>,
        impl_ty: &str,
        program: &Program<'ast>,
        registry: &super::SignatureRegistry,
    ) {
        if func.parameters.iter().any(|p| {
            p.name == "self" && matches!(p.ownership, crate::parser::ast::OwnershipHint::Owned)
        }) {
            return;
        }
        if !self.function_body_has_dispatch_for_loop_needing_mut_self(
            &func.body,
            impl_ty,
            program,
            Some(registry),
        ) {
            return;
        }
        match analyzed.inferred_ownership.get("self") {
            Some(super::OwnershipMode::Borrowed) | None => {
                analyzed
                    .inferred_ownership
                    .insert("self".to_string(), super::OwnershipMode::MutBorrowed);
            }
            _ => {}
        }
    }

    fn function_body_has_dispatch_for_loop_needing_mut_self(
        &self,
        statements: &[&'ast Statement<'ast>],
        impl_ty: &str,
        program: &Program<'ast>,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        statements
            .iter()
            .any(|s| self.statement_tree_has_dispatch_for_loop(s, impl_ty, program, registry))
    }

    fn statement_tree_has_dispatch_for_loop(
        &self,
        stmt: &Statement<'ast>,
        impl_ty: &str,
        program: &Program<'ast>,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match stmt {
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                if self.for_loop_over_self_field_triggers_mut_self(
                    pattern, iterable, body, impl_ty, program, registry,
                ) {
                    return true;
                }
                body.iter().any(|s| {
                    self.statement_tree_has_dispatch_for_loop(s, impl_ty, program, registry)
                })
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(|s| {
                    self.statement_tree_has_dispatch_for_loop(s, impl_ty, program, registry)
                }) || else_block.as_ref().is_some_and(|b| {
                    b.iter().any(|s| {
                        self.statement_tree_has_dispatch_for_loop(s, impl_ty, program, registry)
                    })
                })
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_tree_has_dispatch_for_loop(s, impl_ty, program, registry)),
            Statement::Match { arms, .. } => arms.iter().any(|a| {
                self.statement_tree_has_dispatch_for_loop_in_expr(
                    a.body, impl_ty, program, registry,
                )
            }),
            Statement::Let {
                value, else_block, ..
            } => {
                self.statement_tree_has_dispatch_for_loop_in_expr(value, impl_ty, program, registry)
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| {
                            self.statement_tree_has_dispatch_for_loop(s, impl_ty, program, registry)
                        })
                    })
            }
            Statement::Expression { expr, .. } => {
                self.statement_tree_has_dispatch_for_loop_in_expr(expr, impl_ty, program, registry)
            }
            Statement::Thread { body, .. } | Statement::Async { body, .. } => body
                .iter()
                .any(|s| self.statement_tree_has_dispatch_for_loop(s, impl_ty, program, registry)),
            Statement::Defer { statement, .. } => {
                self.statement_tree_has_dispatch_for_loop(statement, impl_ty, program, registry)
            }
            _ => false,
        }
    }

    fn statement_tree_has_dispatch_for_loop_in_expr(
        &self,
        expr: &Expression<'ast>,
        impl_ty: &str,
        program: &Program<'ast>,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_tree_has_dispatch_for_loop(s, impl_ty, program, registry)),
            _ => false,
        }
    }

    fn for_loop_over_self_field_triggers_mut_self(
        &self,
        pattern: &Pattern,
        iterable: &Expression<'ast>,
        body: &[&Statement<'ast>],
        impl_ty: &str,
        program: &Program<'ast>,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        if self.for_loop_body_mutates_element_of_self_iterable(pattern, iterable, body) {
            return true;
        }

        let Pattern::Identifier(loop_var) = pattern else {
            return false;
        };
        let base_iter = Self::peel_ref_expr(iterable);
        let Expression::FieldAccess { object, field, .. } = base_iter else {
            return false;
        };
        let is_self_root = matches!(
            &**object,
            Expression::Identifier { name, .. } if name == "self"
        );
        if !is_self_root {
            return false;
        }
        let Some(field_ty) = Self::lookup_struct_field_type(program, impl_ty, field.as_str())
        else {
            return false;
        };
        let Some(elem_ty) = Self::type_vec_element(&field_ty) else {
            return false;
        };
        let peeled = Self::peel_box_and_ref_type(&elem_ty);
        body.iter()
            .any(|s| self.stmt_calls_mut_dispatch_on_var(s, loop_var, peeled, registry))
    }

    fn peel_ref_expr<'e>(expr: &'e Expression<'ast>) -> &'e Expression<'ast> {
        match expr {
            Expression::Unary {
                op: UnaryOp::Ref | UnaryOp::MutRef,
                operand,
                ..
            } => Self::peel_ref_expr(operand),
            _ => expr,
        }
    }

    fn lookup_struct_field_type(
        program: &Program<'ast>,
        struct_name: &str,
        field: &str,
    ) -> Option<Type> {
        let base = struct_name.split('<').next().unwrap_or(struct_name);
        Self::lookup_struct_field_type_items(&program.items, base, field)
    }

    fn lookup_struct_field_type_items(
        items: &[crate::parser::Item<'ast>],
        struct_base: &str,
        field: &str,
    ) -> Option<Type> {
        for item in items {
            match item {
                crate::parser::Item::Struct { decl, .. } => {
                    let nb = decl.name.split('<').next().unwrap_or(&decl.name);
                    if nb == struct_base {
                        for f in &decl.fields {
                            if f.name == field {
                                return Some(f.field_type.clone());
                            }
                        }
                    }
                }
                crate::parser::Item::Mod { items: inner, .. } => {
                    if let Some(t) = Self::lookup_struct_field_type_items(inner, struct_base, field)
                    {
                        return Some(t);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn type_vec_element(ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some(inner.as_ref().clone()),
            Type::Reference(inner) | Type::MutableReference(inner) => Self::type_vec_element(inner),
            _ => None,
        }
    }

    fn peel_box_and_ref_type(ty: &Type) -> &Type {
        match ty {
            Type::Parameterized(name, args)
                if (name == "Box" || name.ends_with("::Box")) && args.len() == 1 =>
            {
                Self::peel_box_and_ref_type(&args[0])
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::peel_box_and_ref_type(inner.as_ref())
            }
            _ => ty,
        }
    }

    fn trait_method_self_ownership_lookup(
        &self,
        trait_name: &str,
        method: &str,
    ) -> Option<super::OwnershipMode> {
        let from_map = |m: &std::collections::HashMap<String, super::AnalyzedFunction<'ast>>| {
            m.get(method)
                .and_then(|af| af.inferred_ownership.get("self").copied())
        };
        if let Some(m) = self.analyzed_trait_methods.get(trait_name) {
            if let Some(o) = from_map(m) {
                return Some(o);
            }
        }
        let suffix = format!("::{}", trait_name);
        for (k, m) in &self.analyzed_trait_methods {
            if k == trait_name || k.ends_with(&suffix) {
                if let Some(o) = from_map(m) {
                    return Some(o);
                }
            }
        }
        None
    }

    fn type_needs_mut_receiver_for_method(
        &self,
        elem: &Type,
        method: &str,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        let peeled = Self::peel_box_and_ref_type(elem);
        match peeled {
            Type::TraitObject(trait_name) => self
                .trait_method_self_ownership_lookup(trait_name, method)
                .is_some_and(|o| o == super::OwnershipMode::MutBorrowed),
            Type::Custom(type_name) => registry
                .and_then(|r| r.get_signature(&format!("{}::{}", type_name, method)))
                .filter(|s| s.has_self_receiver)
                .and_then(|s| s.param_ownership.first().copied())
                .is_some_and(|o| o == super::OwnershipMode::MutBorrowed),
            _ => false,
        }
    }

    fn stmt_calls_mut_dispatch_on_var(
        &self,
        stmt: &Statement<'ast>,
        loop_var: &str,
        elem: &Type,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expr_calls_mut_dispatch_on_var(expr, loop_var, elem, registry)
            }
            Statement::Let {
                value, else_block, ..
            } => {
                self.expr_calls_mut_dispatch_on_var(value, loop_var, elem, registry)
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| {
                            self.stmt_calls_mut_dispatch_on_var(s, loop_var, elem, registry)
                        })
                    })
            }
            Statement::Assignment { target, value, .. } => {
                self.expr_calls_mut_dispatch_on_var(target, loop_var, elem, registry)
                    || self.expr_calls_mut_dispatch_on_var(value, loop_var, elem, registry)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_calls_mut_dispatch_on_var(expr, loop_var, elem, registry),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_calls_mut_dispatch_on_var(condition, loop_var, elem, registry)
                    || then_block
                        .iter()
                        .any(|s| self.stmt_calls_mut_dispatch_on_var(s, loop_var, elem, registry))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| {
                            self.stmt_calls_mut_dispatch_on_var(s, loop_var, elem, registry)
                        })
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_calls_mut_dispatch_on_var(condition, loop_var, elem, registry)
                    || body
                        .iter()
                        .any(|s| self.stmt_calls_mut_dispatch_on_var(s, loop_var, elem, registry))
            }
            Statement::For { iterable, body, .. } => {
                self.expr_calls_mut_dispatch_on_var(iterable, loop_var, elem, registry)
                    || body
                        .iter()
                        .any(|s| self.stmt_calls_mut_dispatch_on_var(s, loop_var, elem, registry))
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => body
                .iter()
                .any(|s| self.stmt_calls_mut_dispatch_on_var(s, loop_var, elem, registry)),
            Statement::Match { value, arms, .. } => {
                self.expr_calls_mut_dispatch_on_var(value, loop_var, elem, registry)
                    || arms.iter().any(|a| {
                        a.guard.is_some_and(|g| {
                            self.expr_calls_mut_dispatch_on_var(g, loop_var, elem, registry)
                        }) || self.expr_calls_mut_dispatch_on_var(a.body, loop_var, elem, registry)
                    })
            }
            Statement::Defer { statement, .. } => {
                self.stmt_calls_mut_dispatch_on_var(statement, loop_var, elem, registry)
            }
            _ => false,
        }
    }

    fn expr_calls_mut_dispatch_on_var(
        &self,
        expr: &Expression<'ast>,
        loop_var: &str,
        elem: &Type,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == loop_var
                        && self.type_needs_mut_receiver_for_method(elem, method, registry)
                    {
                        return true;
                    }
                }
                if self.expr_calls_mut_dispatch_on_var(object, loop_var, elem, registry) {
                    return true;
                }
                arguments
                    .iter()
                    .any(|(_, a)| self.expr_calls_mut_dispatch_on_var(a, loop_var, elem, registry))
            }
            Expression::Binary { left, right, .. } => {
                self.expr_calls_mut_dispatch_on_var(left, loop_var, elem, registry)
                    || self.expr_calls_mut_dispatch_on_var(right, loop_var, elem, registry)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_calls_mut_dispatch_on_var(function, loop_var, elem, registry)
                    || arguments.iter().any(|(_, a)| {
                        self.expr_calls_mut_dispatch_on_var(a, loop_var, elem, registry)
                    })
            }
            Expression::Unary { operand, .. } => {
                self.expr_calls_mut_dispatch_on_var(operand, loop_var, elem, registry)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.stmt_calls_mut_dispatch_on_var(s, loop_var, elem, registry)),
            Expression::FieldAccess { object, .. } => {
                self.expr_calls_mut_dispatch_on_var(object, loop_var, elem, registry)
            }
            Expression::Index { object, index, .. } => {
                self.expr_calls_mut_dispatch_on_var(object, loop_var, elem, registry)
                    || self.expr_calls_mut_dispatch_on_var(index, loop_var, elem, registry)
            }
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expr_calls_mut_dispatch_on_var(v, loop_var, elem, registry)),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| self.expr_calls_mut_dispatch_on_var(e, loop_var, elem, registry)),
            Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
                self.expr_calls_mut_dispatch_on_var(expr, loop_var, elem, registry)
            }
            Expression::Closure { body, .. } => {
                self.expr_calls_mut_dispatch_on_var(body, loop_var, elem, registry)
            }
            Expression::Cast { expr, .. } => {
                self.expr_calls_mut_dispatch_on_var(expr, loop_var, elem, registry)
            }
            Expression::Range { start, end, .. } => {
                self.expr_calls_mut_dispatch_on_var(start, loop_var, elem, registry)
                    || self.expr_calls_mut_dispatch_on_var(end, loop_var, elem, registry)
            }
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                self.expr_calls_mut_dispatch_on_var(k, loop_var, elem, registry)
                    || self.expr_calls_mut_dispatch_on_var(v, loop_var, elem, registry)
            }),
            Expression::MacroInvocation { args, .. } => args
                .iter()
                .any(|a| self.expr_calls_mut_dispatch_on_var(a, loop_var, elem, registry)),
            Expression::ChannelSend { channel, value, .. } => {
                self.expr_calls_mut_dispatch_on_var(channel, loop_var, elem, registry)
                    || self.expr_calls_mut_dispatch_on_var(value, loop_var, elem, registry)
            }
            Expression::ChannelRecv { channel, .. } => {
                self.expr_calls_mut_dispatch_on_var(channel, loop_var, elem, registry)
            }
            _ => false,
        }
    }
}
