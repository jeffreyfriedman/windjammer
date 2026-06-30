//! Detection of method calls that require `&mut self` (statements and expressions).
use std::collections::HashSet;

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
                        // THE WINDJAMMER WAY: Check if this method requires &mut self
                        // 1. Check hardcoded stdlib mutating methods
                        if self.is_mutating_method(method) {
                            return true;
                        }

                        // User-defined methods in the current impl block take priority
                        // over stdlib name collisions (e.g., Logger::log vs f64::log).
                        // Check current_impl_functions BEFORE the known-readonly gate.
                        if let Some(impl_functions) = &self.current_impl_functions {
                            if let Some(called_func) = impl_functions.get(method) {
                                if self.function_modifies_self_fields_recursive(
                                    called_func,
                                    registry,
                                    visited,
                                ) {
                                    return true;
                                }
                            }
                        }

                        // Registry stores unqualified method names; unrelated types' `len`/`get`/...
                        // can collide (e.g. safe_buffers::len as &mut self). Read-only std patterns
                        // must never be treated as mutating via that alias.
                        if !Self::is_known_readonly_method(method) {
                            // Check signature registry (has analyzed ownership from previous passes)
                            if let Some(reg) = registry {
                                if let Some(sig) = reg.get_signature(method) {
                                    if sig.has_self_receiver {
                                        if let Some(&ownership) = sig.param_ownership.first() {
                                            if matches!(
                                                ownership,
                                                super::OwnershipMode::MutBorrowed
                                            ) {
                                                return true;
                                            }
                                        }
                                    }
                                }
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
    /// where method requires &mut self. Checks both stdlib list and signature registry.
    pub(crate) fn expression_is_self_field_mutating_method_call(
        &self,
        object: &Expression<'ast>,
        method: &str,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        let traces = self.expression_traces_to_self(object);
        if !traces {
            return false;
        }

        if Self::is_known_readonly_method(method) {
            return false;
        }

        if self.is_mutating_method(method) {
            return true;
        }

        // Check methods in current impl block (same-file, different struct)
        if let Some(impl_functions) = &self.current_impl_functions {
            if let Some(called_func) = impl_functions.get(method) {
                if self.function_modifies_self_fields_recursive(called_func, registry, visited) {
                    return true;
                }
            }
        }

        // Cross-type registry lookup: if the method exists in the registry
        // and takes &mut self, it's a mutating call
        if let Some(reg) = registry {
            if let Some(sig) = reg.get_signature(method) {
                if sig.has_self_receiver {
                    if let Some(&ownership) = sig.param_ownership.first() {
                        if matches!(ownership, super::OwnershipMode::MutBorrowed) {
                            return true;
                        }
                    }
                }
            }
        }

        // Dogfooding: `self.patrol.update_wait()` must resolve `PatrolConfig::update_wait`, not another
        // type's `update_wait` from the unqualified registry / same-impl map.
        if let (Some(ctx), Some(reg)) = (&self.self_impl_context, registry) {
            if let Some(receiver_ty) = self.static_value_type_of_self_rooted_expr(
                ctx.program(),
                &ctx.impl_type_base,
                object,
            ) {
                if let Some(base) = Self::type_base_for_qualified_sig_lookup(&receiver_ty) {
                    let key = format!("{}::{}", base, method);
                    if let Some(sig) = reg.get_signature(&key) {
                        if sig.has_self_receiver {
                            if let Some(&ownership) = sig.param_ownership.first() {
                                if matches!(ownership, super::OwnershipMode::MutBorrowed) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
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

    /// Check if an expression tree contains method calls on any of the given
    /// variable names where the method requires `&mut self`.
    fn body_calls_mutating_method_on_vars(
        &self,
        expr: &Expression<'ast>,
        var_names: &[String],
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
                        if self.is_mutating_method(method) {
                            return true;
                        }
                        if !Self::is_known_readonly_method(method) {
                            if let Some(reg) = registry {
                                let suffix = format!("::{}", method);
                                for (key, sig) in reg.all_signatures() {
                                    if key.ends_with(&suffix)
                                        && sig.has_self_receiver
                                        && sig
                                            .param_ownership
                                            .first()
                                            .is_some_and(|o| {
                                                matches!(o, super::OwnershipMode::MutBorrowed)
                                            })
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                self.body_calls_mutating_method_on_vars(object, var_names, registry, visited)
                    || arguments.iter().any(|(_, arg)| {
                        self.body_calls_mutating_method_on_vars(arg, var_names, registry, visited)
                    })
            }
            Expression::Block { statements, .. } => {
                statements.iter().any(|s| {
                    self.stmt_calls_mutating_method_on_vars(s, var_names, registry, visited)
                })
            }
            Expression::Call { arguments, function, .. } => {
                self.body_calls_mutating_method_on_vars(function, var_names, registry, visited)
                    || arguments.iter().any(|(_, arg)| {
                        self.body_calls_mutating_method_on_vars(
                            arg, var_names, registry, visited,
                        )
                    })
            }
            Expression::Unary { operand, .. } => {
                self.body_calls_mutating_method_on_vars(operand, var_names, registry, visited)
            }
            Expression::Binary { left, right, .. } => {
                self.body_calls_mutating_method_on_vars(left, var_names, registry, visited)
                    || self.body_calls_mutating_method_on_vars(right, var_names, registry, visited)
            }
            _ => false,
        }
    }

    fn stmt_calls_mutating_method_on_vars(
        &self,
        stmt: &Statement<'_>,
        var_names: &[String],
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.body_calls_mutating_method_on_vars(expr, var_names, registry, visited)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.body_calls_mutating_method_on_vars(condition, var_names, registry, visited)
                    || then_block.iter().any(|s| {
                        self.stmt_calls_mutating_method_on_vars(s, var_names, registry, visited)
                    })
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| {
                            self.stmt_calls_mutating_method_on_vars(
                                s, var_names, registry, visited,
                            )
                        })
                    })
            }
            Statement::Return { value, .. } => value.is_some_and(|v| {
                self.body_calls_mutating_method_on_vars(v, var_names, registry, visited)
            }),
            _ => false,
        }
    }
}
