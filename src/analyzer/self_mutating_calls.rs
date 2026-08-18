//! Detection of method calls that require `&mut self` (statements and expressions).
use std::collections::{HashMap, HashSet};

use crate::parser::*;

use super::Analyzer;
impl<'ast> Analyzer<'ast> {
    /// Check if function calls methods on self that require &mut self
    #[allow(dead_code)]
    pub(crate) fn function_calls_mutating_self_methods(&self, func: &FunctionDecl) -> bool {
        let mut visited = HashSet::new();
        self.function_calls_mutating_self_methods_with_registry(func, None, &mut visited)
    }

    /// Check if function calls methods on self that require &mut self (with registry)
    pub(crate) fn function_calls_mutating_self_methods_with_registry(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        for stmt in &func.body {
            if self.statement_calls_mutating_self_methods(stmt, registry, visited) {
                return true;
            }
        }
        false
    }

    /// Check if statement calls methods on self that require &mut self
    pub(crate) fn statement_calls_mutating_self_methods(
        &self,
        stmt: &Statement,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        thread_local! {
            static DEPTH_S: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        }
        let cur = DEPTH_S.with(|d| {
            let v = d.get();
            d.set(v + 1);
            v
        });
        if cur > 1000 {
            DEPTH_S.with(|d| d.set(d.get() - 1));
            return false;
        }
        let result = match stmt {
            Statement::Expression { expr, .. } => {
                self.expression_calls_mutating_self_methods(expr, registry, visited)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_calls_mutating_self_methods(condition, registry, visited)
                    || then_block
                        .iter()
                        .any(|s| self.statement_calls_mutating_self_methods(s, registry, visited))
                    || else_block.as_ref().is_some_and(|block| {
                        block.iter().any(|s| {
                            self.statement_calls_mutating_self_methods(s, registry, visited)
                        })
                    })
            }
            Statement::While { body, .. } => body
                .iter()
                .any(|s| self.statement_calls_mutating_self_methods(s, registry, visited)),
            Statement::For { iterable, body, .. } => {
                self.expression_calls_mutating_self_methods(iterable, registry, visited)
                    || body
                        .iter()
                        .any(|s| self.statement_calls_mutating_self_methods(s, registry, visited))
            }
            Statement::Let { value, .. } => {
                self.expression_calls_mutating_self_methods(value, registry, visited)
            }
            Statement::Match { value, arms, .. } => {
                self.expression_calls_mutating_self_methods(value, registry, visited)
                    || arms.iter().any(|arm| {
                        self.expression_calls_mutating_self_methods(arm.body, registry, visited)
                    })
                    || (self.expression_traces_to_self(value)
                        && arms.iter().any(|arm| {
                            let bound_vars = Self::collect_pattern_bindings(&arm.pattern);
                            !bound_vars.is_empty()
                                && self.body_calls_mutating_method_on_vars(
                                    arm.body,
                                    &bound_vars,
                                    &self.infer_match_arm_binding_type_bases(value, &arm.pattern),
                                    registry,
                                    visited,
                                )
                        }))
            }
            _ => false,
        };
        DEPTH_S.with(|d| d.set(d.get() - 1));
        result
    }

    /// Check if expression calls methods on self that require &mut self
    pub(crate) fn expression_calls_mutating_self_methods(
        &self,
        expr: &Expression,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        thread_local! {
            static DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        }
        let cur = DEPTH.with(|d| {
            let v = d.get();
            d.set(v + 1);
            v
        });
        if cur > 1000 {
            DEPTH.with(|d| d.set(d.get() - 1));
            return false;
        }
        let result = self.expression_calls_mutating_self_methods_inner(expr, registry, visited);
        DEPTH.with(|d| d.set(d.get() - 1));
        result
    }

    pub(crate) fn expression_calls_mutating_self_methods_inner(
        &self,
        expr: &Expression,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Check if calling a method on self (not self.field, just self)
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        // User-defined methods in the current impl block take priority
                        // over stdlib name collisions (e.g., Logger::log vs f64::log).
                        if let Some(impl_functions) = &self.current_impl_functions {
                            if let Some(called_func) = impl_functions.get(method) {
                                if self.function_modifies_self_fields_recursive(
                                    called_func,
                                    registry,
                                    visited,
                                ) {
                                    return true;
                                }
                                if let Some(reg) = registry {
                                    let callee_self = self
                                        .infer_impl_self_receiver_ownership_inner(
                                            called_func,
                                            reg,
                                            visited,
                                        );
                                    return matches!(
                                        callee_self,
                                        super::OwnershipMode::MutBorrowed
                                    );
                                }
                                return false;
                            }
                        }

                        if let Some(reg) = registry {
                            let receiver = self
                                .self_impl_context
                                .as_ref()
                                .map(|ctx| ctx.impl_type_base.as_str());
                            if super::stdlib_method_traits::is_known_readonly_qualified(
                                method, receiver, reg,
                            ) {
                                return false;
                            }
                            if super::stdlib_method_traits::method_mutates_receiver_qualified(
                                method, receiver, reg,
                            ) {
                                return true;
                            }
                        }
                    }
                }

                // Cross-type mutation propagation via self.field.method()
                if self.expression_is_self_field_mutating_method_call(
                    object, method, registry, visited,
                ) {
                    return true;
                }

                // Recurse into object for chained calls: self.nodes.get_mut(id).unwrap()
                if self.expression_calls_mutating_self_methods(object, registry, visited) {
                    return true;
                }

                // Recurse into arguments to find nested mutation patterns
                for (_, arg) in arguments {
                    if self.expression_calls_mutating_self_methods(arg, registry, visited) {
                        return true;
                    }
                }

                false
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_calls_mutating_self_methods(s, registry, visited)),
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // TDD FIX: Check if self.field is passed to a function expecting &mut
                // e.g. handle_player_input(self.game.player, delta_time) needs &mut self
                if let Some(reg) = registry {
                    if let Some(func_name) = self.call_function_name(function) {
                        if let Some(sig) = reg.get_signature(func_name) {
                            for (arg_idx, (_, arg)) in arguments.iter().enumerate() {
                                if self.expression_traces_to_self(arg) {
                                    // Self-field passed as argument - check if callee expects &mut
                                    let param_idx = if sig.has_self_receiver {
                                        arg_idx + 1
                                    } else {
                                        arg_idx
                                    };
                                    if let Some(&ownership) = sig.param_ownership.get(param_idx) {
                                        if matches!(ownership, super::OwnershipMode::MutBorrowed) {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Recurse into arguments for nested patterns
                arguments.iter().any(|(_, arg)| {
                    self.expression_calls_mutating_self_methods(arg, registry, visited)
                })
            }
            Expression::Unary { operand, .. } => {
                self.expression_calls_mutating_self_methods(operand, registry, visited)
            }
            Expression::Binary { left, right, .. } => {
                self.expression_calls_mutating_self_methods(left, registry, visited)
                    || self.expression_calls_mutating_self_methods(right, registry, visited)
            }
            Expression::Index { object, index, .. } => {
                self.expression_calls_mutating_self_methods(object, registry, visited)
                    || self.expression_calls_mutating_self_methods(index, registry, visited)
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_calls_mutating_self_methods(object, registry, visited)
            }
            Expression::Cast { expr, .. } => {
                self.expression_calls_mutating_self_methods(expr, registry, visited)
            }
            _ => false,
        }
    }

    /// Extract function name from a Call's function expression (Identifier or FieldAccess)
    pub(crate) fn call_function_name<'a>(&self, expr: &'a Expression<'a>) -> Option<&'a str> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.as_str()),
            Expression::FieldAccess { field, .. } => Some(field.as_str()),
            _ => None,
        }
    }

    /// Check if object.method() is a self.field[.subfield...].method() pattern
    /// where method requires &mut self — type-qualified signature lookup only.
    pub(crate) fn expression_is_self_field_mutating_method_call(
        &self,
        object: &Expression<'ast>,
        method: &str,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if !self.expression_traces_to_self(object) {
            return false;
        }

        let Some(reg) = registry else {
            return false;
        };

        if let Some(impl_functions) = &self.current_impl_functions {
            if let Some(called_func) = impl_functions.get(method) {
                if self.function_modifies_self_fields_recursive(called_func, registry, visited) {
                    return true;
                }
            }
        }

        let receiver_base = self.self_field_call_receiver_type_base(object);
        super::stdlib_method_traits::method_call_mutates_receiver(
            method,
            receiver_base.as_deref(),
            reg,
        )
    }

    pub(crate) fn self_field_call_receiver_type_base(&self, object: &Expression) -> Option<String> {
        let ctx = self.self_impl_context.as_ref()?;
        if let Some(receiver_ty) =
            self.static_value_type_of_self_rooted_expr(ctx.program(), &ctx.impl_type_base, object)
        {
            return Self::type_base_for_qualified_sig_lookup(&receiver_ty);
        }
        let field_name = Self::extract_direct_self_field_name(object)?;
        let field_type = self.struct_field_types_lookup(&ctx.impl_type_base, &field_name)?;
        Self::type_base_for_qualified_sig_lookup(&field_type)
    }

    /// Extract the field name from `self.field` or `self.field.subfield...`.
    /// Returns the immediate field name after `self`.
    fn extract_direct_self_field_name(expr: &Expression) -> Option<String> {
        match expr {
            Expression::FieldAccess { object, field, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        return Some(field.clone());
                    }
                }
                Self::extract_direct_self_field_name(object)
            }
            _ => None,
        }
    }

    /// Look up a field's type from the global struct field type map.
    fn struct_field_types_lookup(
        &self,
        struct_name: &str,
        field_name: &str,
    ) -> Option<crate::parser::Type> {
        if let Some(fields) = self.global_struct_field_types.get(struct_name) {
            if let Some(ty) = fields.get(field_name) {
                return Some(ty.clone());
            }
        }
        // Try unqualified base name
        if let Some(base) = struct_name.rsplit("::").next() {
            if base != struct_name {
                if let Some(fields) = self.global_struct_field_types.get(base) {
                    if let Some(ty) = fields.get(field_name) {
                        return Some(ty.clone());
                    }
                }
            }
        }
        None
    }

    /// Collect variable names bound by a pattern (e.g. `Some(search)` → `["search"]`).
    fn collect_pattern_bindings(pattern: &crate::parser::Pattern) -> Vec<String> {
        let mut names = Vec::new();
        Self::collect_pattern_bindings_inner(pattern, &mut names);
        names
    }

    fn collect_pattern_bindings_inner(pattern: &crate::parser::Pattern, out: &mut Vec<String>) {
        use crate::parser::{EnumPatternBinding, Pattern};
        match pattern {
            Pattern::Identifier(name) => out.push(name.clone()),
            Pattern::MutBinding(name) | Pattern::Ref(name) | Pattern::RefMut(name) => {
                out.push(name.clone());
            }
            Pattern::EnumVariant(_, binding) => match binding {
                EnumPatternBinding::Single(name) => out.push(name.clone()),
                EnumPatternBinding::Tuple(pats) => {
                    for p in pats {
                        Self::collect_pattern_bindings_inner(p, out);
                    }
                }
                EnumPatternBinding::Struct(fields, _) => {
                    for (_, p) in fields {
                        Self::collect_pattern_bindings_inner(p, out);
                    }
                }
                _ => {}
            },
            Pattern::Tuple(pats) | Pattern::Or(pats) => {
                for p in pats {
                    Self::collect_pattern_bindings_inner(p, out);
                }
            }
            Pattern::Reference(inner) => Self::collect_pattern_bindings_inner(inner, out),
            _ => {}
        }
    }

    /// Infer registry lookup bases for variables bound in a match arm pattern when the
    /// scrutinee is rooted at `self` (field access or index).
    fn infer_match_arm_binding_type_bases(
        &self,
        scrutinee: &Expression<'ast>,
        pattern: &Pattern,
    ) -> HashMap<String, String> {
        let mut out = HashMap::new();
        let Some(ctx) = self.self_impl_context.as_ref() else {
            return out;
        };
        let Some(mut scrutinee_ty) = self.static_value_type_of_self_rooted_expr(
            ctx.program(),
            &ctx.impl_type_base,
            scrutinee,
        ) else {
            return out;
        };
        while matches!(scrutinee_ty, Type::Reference(_) | Type::MutableReference(_)) {
            scrutinee_ty = match scrutinee_ty {
                Type::Reference(inner) | Type::MutableReference(inner) => *inner,
                other => other,
            };
        }
        Self::collect_pattern_binding_type_bases(pattern, &scrutinee_ty, &mut out);
        out
    }

    fn collect_pattern_binding_type_bases(
        pattern: &Pattern,
        scrutinee_ty: &Type,
        out: &mut HashMap<String, String>,
    ) {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::EnumVariant(variant, EnumPatternBinding::Single(name))
                if variant == "Some" || variant.ends_with("::Some") =>
            {
                if let Type::Option(inner) = scrutinee_ty {
                    if let Some(base) = Self::type_base_for_qualified_sig_lookup(inner) {
                        out.insert(name.clone(), base);
                    }
                }
            }
            Pattern::EnumVariant(_, EnumPatternBinding::Tuple(pats)) => {
                if let Type::Tuple(types) = scrutinee_ty {
                    for (pat, ty) in pats.iter().zip(types.iter()) {
                        Self::collect_pattern_binding_type_bases(pat, ty, out);
                    }
                }
            }
            Pattern::EnumVariant(_, EnumPatternBinding::Struct(_, _)) => {
                // Struct-like enum variants need variant field metadata the analyzer
                // pass does not yet hold; multipass + qualified lookup on the bound
                // ident still runs when the binding type can be inferred later.
            }
            Pattern::Identifier(name) => {
                if let Some(base) = Self::type_base_for_qualified_sig_lookup(scrutinee_ty) {
                    out.insert(name.clone(), base);
                }
            }
            Pattern::MutBinding(name) | Pattern::Ref(name) | Pattern::RefMut(name) => {
                if let Some(base) = Self::type_base_for_qualified_sig_lookup(scrutinee_ty) {
                    out.insert(name.clone(), base);
                }
            }
            Pattern::Tuple(pats) => {
                if let Type::Tuple(types) = scrutinee_ty {
                    for (pat, ty) in pats.iter().zip(types.iter()) {
                        Self::collect_pattern_binding_type_bases(pat, ty, out);
                    }
                }
            }
            Pattern::Or(alts) => {
                for alt in alts {
                    Self::collect_pattern_binding_type_bases(alt, scrutinee_ty, out);
                }
            }
            Pattern::Reference(inner) => {
                let ref_ty = Type::Reference(Box::new(scrutinee_ty.clone()));
                Self::collect_pattern_binding_type_bases(inner, &ref_ty, out);
            }
            _ => {}
        }
    }

    /// Check if an expression tree contains method calls on any of the given
    /// variable names where the method requires `&mut self`.
    fn body_calls_mutating_method_on_vars(
        &self,
        expr: &Expression<'ast>,
        var_names: &[String],
        binding_type_bases: &HashMap<String, String>,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if var_names.contains(name) {
                        if let Some(reg) = registry {
                            let receiver_base = binding_type_bases.get(name).map(String::as_str);
                            if super::stdlib_method_traits::method_call_mutates_receiver(
                                method,
                                receiver_base,
                                reg,
                            ) {
                                return true;
                            }
                        }
                    }
                }
                self.body_calls_mutating_method_on_vars(
                    object,
                    var_names,
                    binding_type_bases,
                    registry,
                    visited,
                ) || arguments.iter().any(|(_, arg)| {
                    self.body_calls_mutating_method_on_vars(
                        arg,
                        var_names,
                        binding_type_bases,
                        registry,
                        visited,
                    )
                })
            }
            Expression::Block { statements, .. } => statements.iter().any(|s| {
                self.stmt_calls_mutating_method_on_vars(
                    s,
                    var_names,
                    binding_type_bases,
                    registry,
                    visited,
                )
            }),
            Expression::Call {
                arguments,
                function,
                ..
            } => {
                self.body_calls_mutating_method_on_vars(
                    function,
                    var_names,
                    binding_type_bases,
                    registry,
                    visited,
                ) || arguments.iter().any(|(_, arg)| {
                    self.body_calls_mutating_method_on_vars(
                        arg,
                        var_names,
                        binding_type_bases,
                        registry,
                        visited,
                    )
                })
            }
            Expression::Unary { operand, .. } => self.body_calls_mutating_method_on_vars(
                operand,
                var_names,
                binding_type_bases,
                registry,
                visited,
            ),
            Expression::Binary { left, right, .. } => {
                self.body_calls_mutating_method_on_vars(
                    left,
                    var_names,
                    binding_type_bases,
                    registry,
                    visited,
                ) || self.body_calls_mutating_method_on_vars(
                    right,
                    var_names,
                    binding_type_bases,
                    registry,
                    visited,
                )
            }
            _ => false,
        }
    }

    fn stmt_calls_mutating_method_on_vars(
        &self,
        stmt: &Statement<'_>,
        var_names: &[String],
        binding_type_bases: &HashMap<String, String>,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.body_calls_mutating_method_on_vars(
                expr,
                var_names,
                binding_type_bases,
                registry,
                visited,
            ),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.body_calls_mutating_method_on_vars(
                    condition,
                    var_names,
                    binding_type_bases,
                    registry,
                    visited,
                ) || then_block.iter().any(|s| {
                    self.stmt_calls_mutating_method_on_vars(
                        s,
                        var_names,
                        binding_type_bases,
                        registry,
                        visited,
                    )
                }) || else_block.as_ref().is_some_and(|b| {
                    b.iter().any(|s| {
                        self.stmt_calls_mutating_method_on_vars(
                            s,
                            var_names,
                            binding_type_bases,
                            registry,
                            visited,
                        )
                    })
                })
            }
            Statement::Return { value, .. } => value.is_some_and(|v| {
                self.body_calls_mutating_method_on_vars(
                    v,
                    var_names,
                    binding_type_bases,
                    registry,
                    visited,
                )
            }),
            _ => false,
        }
    }
}
