//! Self-analysis methods for the analyzer.
//! Determines whether impl methods need &self, &mut self, or owned self
//! by analyzing field access patterns, mutations, and return types.

use std::collections::HashSet;

use crate::parser::*;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    /// Check if a function modifies self fields (for impl methods)
    #[allow(dead_code)]
    pub(super) fn function_modifies_self_fields(&self, func: &FunctionDecl) -> bool {
        self.function_modifies_self_fields_with_registry(func, None)
    }

    /// Check if a function modifies self fields, with optional registry for cross-type resolution
    pub(super) fn function_modifies_self_fields_with_registry(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        let mut visited = HashSet::new();
        self.function_modifies_self_fields_with_registry_inner(func, registry, &mut visited)
    }

    fn function_modifies_self_fields_with_registry_inner(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if let Some(return_type) = &func.return_type {
            if self.type_is_mut_ref(return_type) {
                return true;
            }
        }

        let calls_mut =
            self.function_calls_mutating_self_methods_with_registry(func, registry, visited);
        if calls_mut {
            return true;
        }

        if self.function_mutates_through_self_option_scrutinee(func, registry) {
            return true;
        }

        for stmt in func.body.iter() {
            if self.statement_modifies_self_fields(stmt, registry, visited) {
                return true;
            }
        }
        false
    }

    /// Recursively check if a function modifies self fields.
    ///
    /// `visited` prevents re-analysis: once a function has been analyzed,
    /// its result is cached. On cycle (A→B→A), returns false — the cycle
    /// itself provides no evidence of mutation. Actual mutations are caught
    /// on non-recursive paths.
    fn function_modifies_self_fields_recursive(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if !visited.insert(func.name.clone()) {
            return false;
        }

        if let Some(return_type) = &func.return_type {
            if self.type_is_mut_ref(return_type) {
                return true;
            }
        }

        func.body
            .iter()
            .any(|stmt| self.statement_modifies_self_fields(stmt, registry, visited))
    }

    /// Check if a type contains a mutable reference (&mut T)
    /// This includes Option<&mut T>, Result<&mut T, E>, Vec<&mut T>, etc.
    fn type_is_mut_ref(&self, ty: &Type) -> bool {
        match ty {
            Type::MutableReference(_) => true,
            Type::Option(inner) | Type::Vec(inner) | Type::Reference(inner) => {
                self.type_is_mut_ref(inner)
            }
            Type::Result(ok, err) => self.type_is_mut_ref(ok) || self.type_is_mut_ref(err),
            Type::Tuple(types) => types.iter().any(|t| self.type_is_mut_ref(t)),
            Type::Parameterized(_, args) => args.iter().any(|t| self.type_is_mut_ref(t)),
            _ => false,
        }
    }

    /// Check if function calls methods on self that require &mut self
    #[allow(dead_code)]
    fn function_calls_mutating_self_methods(&self, func: &FunctionDecl) -> bool {
        let mut visited = HashSet::new();
        self.function_calls_mutating_self_methods_with_registry(func, None, &mut visited)
    }

    /// Check if function calls methods on self that require &mut self (with registry)
    fn function_calls_mutating_self_methods_with_registry(
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
    fn statement_calls_mutating_self_methods(
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
                then_block,
                else_block,
                ..
            } => {
                then_block
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
                // TDD FIX: Match arms can contain mutating method calls (e.g. match choice { 0 => self.companion.adjust_loyalty(-5.0) })
                self.expression_calls_mutating_self_methods(value, registry, visited)
                    || arms.iter().any(|arm| {
                        self.expression_calls_mutating_self_methods(arm.body, registry, visited)
                    })
            }
            _ => false,
        };
        DEPTH_S.with(|d| d.set(d.get() - 1));
        result
    }

    /// Check if expression calls methods on self that require &mut self
    fn expression_calls_mutating_self_methods(
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

    fn expression_calls_mutating_self_methods_inner(
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
            _ => false,
        }
    }

    /// Extract function name from a Call's function expression (Identifier or FieldAccess)
    fn call_function_name<'a>(&self, expr: &'a Expression<'a>) -> Option<&'a str> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.as_str()),
            Expression::FieldAccess { field, .. } => Some(field.as_str()),
            _ => None,
        }
    }

    /// Check if object.method() is a self.field[.subfield...].method() pattern
    /// where method requires &mut self. Checks both stdlib list and signature registry.
    fn expression_is_self_field_mutating_method_call(
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
            if let Some(receiver_ty) = Self::static_value_type_of_self_rooted_expr(
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

    fn type_base_for_qualified_sig_lookup(ty: &Type) -> Option<String> {
        match ty {
            Type::Custom(name) => Some(name.split('<').next()?.to_string()),
            Type::Parameterized(name, _) => Some(name.split('<').next()?.to_string()),
            Type::TraitObject(name) => Some(name.clone()),
            _ => None,
        }
    }

    /// Static type of `self`, `self.f`, `self.f[i]`, ... for registry keys like `FieldType::method`.
    fn static_value_type_of_self_rooted_expr(
        program: &Program<'ast>,
        impl_base: &str,
        expr: &Expression<'ast>,
    ) -> Option<Type> {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => {
                Some(Type::Custom(impl_base.to_string()))
            }
            Expression::FieldAccess { object, field, .. } => {
                let parent_ty =
                    Self::static_value_type_of_self_rooted_expr(program, impl_base, object)?;
                let struct_base = Self::type_base_for_qualified_sig_lookup(&parent_ty)?;
                Self::lookup_struct_field_type(program, &struct_base, field.as_str())
            }
            Expression::Index { object, .. } => {
                let parent_ty =
                    Self::static_value_type_of_self_rooted_expr(program, impl_base, object)?;
                Self::type_vec_element(&parent_ty)
            }
            _ => None,
        }
    }

    /// `for x in self.items` / `&mut self.items` where the body assigns to `x` or `x.field` needs `&mut self`
    /// because codegen emits `for x in &mut self.items`.
    fn for_loop_body_mutates_element_of_self_iterable(
        &self,
        pattern: &Pattern,
        iterable: &Expression<'ast>,
        body: &[&Statement<'ast>],
    ) -> bool {
        // TDD FIX: Handle tuple patterns (id, val) not just simple identifiers
        // Extract ALL bindings from the pattern and check if ANY are mutated
        let loop_vars = match pattern {
            Pattern::Identifier(var) => vec![var.clone()],
            Pattern::Tuple(patterns) => {
                let mut vars = Vec::new();
                for pat in patterns {
                    if let Pattern::Identifier(var) = pat {
                        vars.push(var.clone());
                    }
                }
                vars
            }
            _ => return false,
        };

        let peeled = Self::peel_ref_expr(iterable);
        if !self.expression_traces_to_self(peeled) {
            return false;
        }

        // Check if ANY binding is mutated
        loop_vars.iter().any(|loop_var| {
            body.iter()
                .any(|s| self.statement_tree_mutates_binding(s, loop_var.as_str()))
        })
    }

    fn assignment_target_starts_with_var(expr: &Expression, var: &str) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == var,
            Expression::FieldAccess { object, .. } => {
                Self::assignment_target_starts_with_var(object, var)
            }
            Expression::Index { object, .. } => {
                Self::assignment_target_starts_with_var(object, var)
            }
            // TDD FIX: Handle dereference assignments (*val = ..., *var.field = ...)
            // For: *val = value, need to detect that 'val' is the target variable
            Expression::Unary {
                op: UnaryOp::Deref,
                operand,
                ..
            } => Self::assignment_target_starts_with_var(operand, var),
            _ => false,
        }
    }

    /// Whether `stmt` mutates `var` or a field/index chain rooted at `var` (loop element patterns).
    fn statement_tree_mutates_binding(&self, stmt: &Statement, var: &str) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                Self::assignment_target_starts_with_var(target, var)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_tree_mutates_binding(s, var))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_tree_mutates_binding(s, var))
                    })
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_tree_mutates_binding(s, var)),
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                self.for_loop_body_mutates_element_of_self_iterable(pattern, iterable, body)
                    || body
                        .iter()
                        .any(|s| self.statement_tree_mutates_binding(s, var))
            }
            Statement::Match { arms, .. } => arms
                .iter()
                .any(|arm| self.expression_tree_mutates_binding(arm.body, var)),
            Statement::Expression { expr, .. } => self.expression_tree_mutates_binding(expr, var),
            Statement::Let {
                value, else_block, ..
            } => {
                self.expression_tree_mutates_binding(value, var)
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_tree_mutates_binding(s, var))
                    })
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_tree_mutates_binding(expr, var),
            _ => false,
        }
    }

    fn expression_tree_mutates_binding(&self, expr: &Expression, var: &str) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_tree_mutates_binding(s, var)),
            _ => false,
        }
    }

    /// Check if an expression traces back to `self` through a chain of field accesses.
    /// Returns true for: self.field, self.field.subfield, self.field[i], etc.
    fn expression_traces_to_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    name == "self"
                } else {
                    self.expression_traces_to_self(object)
                }
            }
            // TDD FIX: self.factions[i].method() - Index on self.field traces to self
            Expression::Index { object, .. } => self.expression_traces_to_self(object),
            _ => false,
        }
    }

    /// Check if a statement modifies self fields
    fn statement_modifies_self_fields(
        &self,
        stmt: &Statement,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        thread_local! {
            static DEPTH_SM: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        }
        let cur = DEPTH_SM.with(|d| {
            let v = d.get();
            d.set(v + 1);
            v
        });
        if cur > 1000 {
            DEPTH_SM.with(|d| d.set(d.get() - 1));
            return false;
        }
        let result = match stmt {
            Statement::Assignment { target, .. } => {
                self.expression_is_self_field_access(target)
                    || self.expression_is_self_field_index_access(target)
            }
            Statement::Expression { expr, .. } => {
                self.expression_mutates_self_fields(expr, registry, visited)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                // THE WINDJAMMER WAY: Check condition for mutations!
                self.expression_mutates_self_fields(condition, registry, visited)
                    || then_block
                        .iter()
                        .any(|s| self.statement_modifies_self_fields(s, registry, visited))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_modifies_self_fields(s, registry, visited))
                    })
            }
            Statement::While { body, .. } => body
                .iter()
                .any(|s| self.statement_modifies_self_fields(s, registry, visited)),
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                // THE WINDJAMMER WAY: Check BOTH the iterable AND the body!
                self.expression_mutates_self_fields(iterable, registry, visited)
                    || self.for_loop_body_mutates_element_of_self_iterable(pattern, iterable, body)
                    || body
                        .iter()
                        .any(|s| self.statement_modifies_self_fields(s, registry, visited))
            }
            Statement::Match { value, arms, .. } => {
                // THE WINDJAMMER WAY: Check match value for mutations!
                self.expression_mutates_self_fields(value, registry, visited)
                    || arms.iter().any(|arm| {
                        self.expression_contains_self_field_mutations(arm.body, registry, visited)
                    })
            }
            Statement::Return { value, .. } => value
                .as_ref()
                .is_some_and(|expr| self.expression_mutates_self_fields(expr, registry, visited)),
            Statement::Let { value, .. } => {
                self.expression_mutates_self_fields(value, registry, visited)
            }
            _ => false,
        };
        DEPTH_SM.with(|d| d.set(d.get() - 1));
        result
    }

    /// Check if an expression contains self field mutations (for match arms and blocks)
    fn expression_contains_self_field_mutations(
        &self,
        expr: &Expression,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_modifies_self_fields(s, registry, visited)),
            Expression::MethodCall { .. } => {
                self.expression_mutates_self_fields(expr, registry, visited)
            }
            _ => false,
        }
    }

    /// Check if expression is a self field access (self.field)
    pub(super) fn expression_is_self_field_access(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                match &**object {
                    Expression::Identifier { name, .. } if name == "self" => true,
                    Expression::FieldAccess { .. } => self.expression_is_self_field_access(object),
                    // CRITICAL FIX: Handle index expressions like self.children[i].field
                    Expression::Index { .. } => self.expression_is_self_field_index_access(object),
                    _ => false,
                }
            }
            // Handle self.slots[i] — Index wrapping a self field access
            Expression::Index { object, .. } => {
                self.expression_is_self_field_access(object)
                    || self.expression_is_self_field_index_access(object)
            }
            _ => false,
        }
    }

    /// Check if expression is an index access on a self field (self.field[index] or self.field[i][j])
    fn expression_is_self_field_index_access(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Index { object, .. } => {
                self.expression_is_self_field_access(object)
                    || self.expression_is_self_field_index_access(object)
            }
            _ => false,
        }
    }

    /// Check if expression mutates self fields
    fn expression_mutates_self_fields(
        &self,
        expr: &Expression,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        thread_local! {
            static DEPTH_EM: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        }
        let cur = DEPTH_EM.with(|d| {
            let v = d.get();
            d.set(v + 1);
            v
        });
        if cur > 1000 {
            DEPTH_EM.with(|d| d.set(d.get() - 1));
            return false;
        }
        let result = self.expression_mutates_self_fields_inner(expr, registry, visited);
        DEPTH_EM.with(|d| d.set(d.get() - 1));
        result
    }

    fn expression_mutates_self_fields_inner(
        &self,
        expr: &Expression,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                let is_field = self.expression_is_self_field_access(object);
                let is_mut_method = self.is_mutating_method(method);
                if is_field && is_mut_method {
                    return true;
                }

                if self.expression_is_self_field_index_access(object)
                    && self.is_mutating_method(method)
                {
                    return true;
                }

                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        if self.is_mutating_method(method) {
                            return true;
                        }
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
                        if !Self::is_known_readonly_method(method) {
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

                self.expression_is_self_field_mutating_method_call(
                    object, method, registry, visited,
                )
            }
            Expression::Unary { op, operand, .. } => {
                if matches!(op, crate::parser::UnaryOp::MutRef) {
                    self.expression_is_self_field_access(operand)
                } else {
                    self.expression_mutates_self_fields(operand, registry, visited)
                }
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_modifies_self_fields(s, registry, visited)),
            _ => false,
        }
    }

    /// Check if a function returns Self (for builder pattern detection)
    pub(super) fn function_returns_self(&self, func: &FunctionDecl) -> bool {
        use crate::parser::{Statement, Type};

        // "returns Self" means the return type matches the parent type
        // (the type that `self` belongs to), indicating a builder pattern
        let parent_type = match &func.parent_type {
            Some(name) => name,
            None => return false,
        };
        let return_type_name = match &func.return_type {
            Some(Type::Custom(name)) if name == parent_type => name,
            _ => return false,
        };

        if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => self.expression_returns_self_type(expr, return_type_name),
                Statement::Expression { expr, .. } => {
                    self.expression_returns_self_type(expr, return_type_name)
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// True when the function returns a non-Copy `self.field` expression (last statement).
    /// Used only for **declared** `self` with `Inferred` ownership (`fn f(self)`): moving a field
    /// out requires owned `self`. Omitted-receiver methods (`fn g() { self.x }`) use a different
    /// path and treat final `self.field` as a `&self` getter (codegen inserts `.clone()`).
    pub(super) fn function_returns_non_copy_self_field(&self, func: &FunctionDecl) -> bool {
        use crate::parser::Statement;

        let return_type = match &func.return_type {
            Some(t) => t,
            None => return false,
        };

        if self.is_copy_type(return_type) {
            return false;
        }

        if !func.parameters.iter().any(|p| p.name == "self") {
            return false;
        }

        if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => self.expression_is_self_field_access(expr),
                Statement::Expression { expr, .. } => self.expression_is_self_field_access(expr),
                _ => false,
            }
        } else {
            false
        }
    }

    /// Check if self is moved into a returned struct literal (e.g., `OtherType { field: self }`)
    /// or returned directly as a value. This means self must be consumed (owned).
    pub(super) fn function_moves_self_into_return(&self, func: &FunctionDecl) -> bool {
        use crate::parser::Statement;
        if let Some(last_stmt) = func.body.last() {
            let expr = match last_stmt {
                Statement::Return { value: Some(e), .. } => Some(e),
                Statement::Expression { expr, .. } => Some(expr),
                _ => None,
            };
            if let Some(expr) = expr {
                return self.expression_consumes_self(expr);
            }
        }
        false
    }

    fn expression_consumes_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| matches!(v, Expression::Identifier { name, .. } if name == "self")),
            _ => false,
        }
    }

    /// Check if `self` is used as a match scrutinee (match self { ... })
    /// AND the match arms actually consume values from self.
    ///
    /// TDD FIX for E0606: match self { Value::Int(v) => v as f32, ... }
    /// If match arms return/use bound variables, self must be owned.
    /// If match arms only return literals, &self is sufficient.
    ///
    /// Examples:
    ///   match self { Value::Int(v) => v as f32 } → needs `self` (v is used)
    ///   match self { Condition::HasItem(_) => false } → needs `&self` (literal returned)
    pub(super) fn function_matches_on_self(&self, func: &FunctionDecl) -> bool {
        for stmt in &func.body {
            if self.statement_matches_on_self_consuming(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if function iterates over self.field and calls consuming methods on elements.
    ///
    /// TDD FIX for E0507: for cond in self.conditions { cond.check() }
    /// If loop elements call methods that consume self, the outer method must consume self.
    ///
    /// Example:
    ///   for item in self.items { item.consume() } → needs `self` (owned)
    pub(super) fn function_consumes_self_field_elements(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        for stmt in &func.body {
            if self.statement_consumes_self_field_elements(stmt, registry) {
                return true;
            }
        }
        false
    }

    fn statement_consumes_self_field_elements(
        &self,
        stmt: &Statement,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match stmt {
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                // Check if iterable is self.field (e.g., self.conditions)
                if !self.expression_is_self_field(iterable) {
                    return false;
                }

                // Get the loop variable name from pattern
                let loop_var = match pattern {
                    Pattern::Identifier(name) => name.clone(),
                    _ => return false,
                };

                // Check if loop body calls consuming methods on loop_var
                body.iter()
                    .any(|s| self.statement_calls_consuming_method_on_var(s, &loop_var, registry))
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_consumes_self_field_elements(s, registry))
                    || else_block.as_ref().is_some_and(|body| {
                        body.iter()
                            .any(|s| self.statement_consumes_self_field_elements(s, registry))
                    })
            }
            Statement::While { body, .. }
            | Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => body
                .iter()
                .any(|s| self.statement_consumes_self_field_elements(s, registry)),
            _ => false,
        }
    }

    fn expression_is_self_field(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }
    }

    fn statement_calls_consuming_method_on_var(
        &self,
        stmt: &Statement,
        var_name: &str,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            }
            | Statement::Let { value: expr, .. }
            | Statement::Assignment { value: expr, .. } => {
                self.expression_calls_consuming_method_on_var(expr, var_name, registry)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_calls_consuming_method_on_var(condition, var_name, registry)
                    || then_block.iter().any(|s| {
                        self.statement_calls_consuming_method_on_var(s, var_name, registry)
                    })
                    || else_block.as_ref().is_some_and(|body| {
                        body.iter().any(|s| {
                            self.statement_calls_consuming_method_on_var(s, var_name, registry)
                        })
                    })
            }
            _ => false,
        }
    }

    fn expression_calls_consuming_method_on_var(
        &self,
        expr: &Expression,
        var_name: &str,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object, method: _, ..
            } => {
                // Check if object is our loop variable
                if let Expression::Identifier { name, .. } = &**object {
                    if name == var_name {
                        // Check if the method consumes self
                        // We need to look up the method's signature
                        // For now, use heuristics: methods that match on self usually consume
                        return true; // Conservative: assume method consumes
                    }
                }
                false
            }
            Expression::Binary { left, right, .. } => {
                self.expression_calls_consuming_method_on_var(left, var_name, registry)
                    || self.expression_calls_consuming_method_on_var(right, var_name, registry)
            }
            Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
                self.expression_calls_consuming_method_on_var(arg, var_name, registry)
            }),
            Expression::Unary { operand, .. } => {
                self.expression_calls_consuming_method_on_var(operand, var_name, registry)
            }
            _ => false,
        }
    }

    fn statement_matches_on_self_consuming(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Match { value, arms, .. } => {
                // Check if the match scrutinee is `self` or `&self`
                if self.expression_is_self_or_ref_self(value) {
                    // Now check if the match arms actually consume values
                    return self.match_arms_consume_bound_values(arms);
                }
                // Recursively check match arm bodies (which are expressions)
                arms.iter()
                    .any(|arm| self.expression_contains_match_on_self_consuming(arm.body))
            }
            Statement::Let { value: expr, .. }
            | Statement::Assignment { value: expr, .. }
            | Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => self.expression_contains_match_on_self_consuming(expr),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_contains_match_on_self_consuming(condition)
                    || then_block
                        .iter()
                        .any(|s| self.statement_matches_on_self_consuming(s))
                    || else_block.as_ref().is_some_and(|body| {
                        body.iter()
                            .any(|s| self.statement_matches_on_self_consuming(s))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_contains_match_on_self_consuming(condition)
                    || body
                        .iter()
                        .any(|s| self.statement_matches_on_self_consuming(s))
            }
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_matches_on_self_consuming(s)),
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => body
                .iter()
                .any(|s| self.statement_matches_on_self_consuming(s)),
            Statement::Defer { statement, .. } => {
                self.statement_matches_on_self_consuming(statement)
            }
            _ => false,
        }
    }

    /// Check if match arms consume bound values (return them, cast them, etc.)
    /// vs. just returning literals without using the bindings.
    fn match_arms_consume_bound_values(&self, arms: &[crate::parser::MatchArm]) -> bool {
        for arm in arms {
            // Get all variable names bound in the pattern
            let bound_vars = self.pattern_bound_variables(&arm.pattern);

            // Check if the arm body uses any of these variables in a consuming way
            if self.expression_uses_variables_consuming(arm.body, &bound_vars) {
                return true;
            }
        }

        false
    }

    fn pattern_bound_variables(&self, pattern: &Pattern) -> Vec<String> {
        use crate::parser::EnumPatternBinding;

        let mut vars = Vec::new();
        match pattern {
            Pattern::Identifier(name) if !name.starts_with('_') => {
                vars.push(name.clone());
            }
            Pattern::EnumVariant(_, binding) => match binding {
                EnumPatternBinding::Single(name) if !name.starts_with('_') => {
                    vars.push(name.clone());
                }
                EnumPatternBinding::Tuple(patterns) => {
                    for p in patterns {
                        vars.append(&mut self.pattern_bound_variables(p));
                    }
                }
                EnumPatternBinding::Struct(fields, _) => {
                    for (_, p) in fields {
                        vars.append(&mut self.pattern_bound_variables(p));
                    }
                }
                _ => {}
            },
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    vars.append(&mut self.pattern_bound_variables(p));
                }
            }
            Pattern::Ref(name) | Pattern::RefMut(name) if !name.starts_with('_') => {
                vars.push(name.clone());
            }
            _ => {}
        }
        vars
    }

    fn expression_uses_variables_consuming(&self, expr: &Expression, vars: &[String]) -> bool {
        use crate::parser::Expression;

        match expr {
            // If the expression is just an identifier from our bound vars, it's consumed
            Expression::Identifier { name, .. } if vars.contains(name) => true,

            // Casts consume the value
            Expression::Cast { expr: inner, .. } => {
                self.expression_uses_variables_consuming(inner, vars)
            }

            // Binary operations might consume (depends on the variable being used)
            Expression::Binary { left, right, .. } => {
                self.expression_uses_variables_consuming(left, vars)
                    || self.expression_uses_variables_consuming(right, vars)
            }

            // Method calls consume self if self is a bound var
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_uses_variables_consuming(object, vars)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_uses_variables_consuming(arg, vars))
            }

            // Function calls consume arguments
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_uses_variables_consuming(arg, vars)),

            // Field access on a bound var consumes if the field is non-Copy
            Expression::FieldAccess { object, .. } => {
                self.expression_uses_variables_consuming(object, vars)
            }

            // Literals don't consume anything
            Expression::Literal { .. } => false,

            // Blocks: check last expression
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_uses_variables_consuming(s, vars)),

            _ => false,
        }
    }

    fn statement_uses_variables_consuming(&self, stmt: &Statement, vars: &[String]) -> bool {
        match stmt {
            Statement::Return {
                value: Some(expr), ..
            }
            | Statement::Expression { expr, .. } => {
                self.expression_uses_variables_consuming(expr, vars)
            }
            _ => false,
        }
    }

    fn expression_contains_match_on_self_consuming(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_matches_on_self_consuming(s)),
            Expression::Binary { left, right, .. } => {
                self.expression_contains_match_on_self_consuming(left)
                    || self.expression_contains_match_on_self_consuming(right)
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_contains_match_on_self_consuming(arg)),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_contains_match_on_self_consuming(object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_contains_match_on_self_consuming(arg))
            }
            _ => false,
        }
    }

    fn expression_is_self_or_ref_self(&self, expr: &Expression) -> bool {
        use crate::parser::UnaryOp;
        match expr {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::Unary {
                op: UnaryOp::Ref | UnaryOp::MutRef,
                operand,
                ..
            } => {
                matches!(&**operand, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }
    }

    /// Check if ANY statement in the function body moves a non-Copy field out of self.
    /// Walks the entire body for patterns like:
    ///   - `let x = self.field` (where field is non-Copy)
    ///   - `Foo { field: self.field }` (struct literal with non-Copy self field)
    ///   - `let mut x = self.field` (assignment from non-Copy self field)
    ///
    /// Direct `return self.field` / trailing `self.field` as implicit return are excluded: codegen
    /// emits `.clone()` for `&self` receivers (same rule as read-only getters).
    pub(super) fn function_body_moves_non_copy_self_fields(&self, func: &FunctionDecl) -> bool {
        for stmt in &func.body {
            if self.statement_moves_non_copy_self_field(stmt) {
                return true;
            }
        }
        false
    }

    fn statement_moves_non_copy_self_field(&self, stmt: &Statement) -> bool {
        use crate::parser::Statement;
        match stmt {
            Statement::Let { value, .. } => self.expression_moves_non_copy_self_field(value),
            Statement::Assignment { value, .. } => self.expression_moves_non_copy_self_field(value),
            Statement::Expression { expr, .. } => {
                if self.expression_is_self_field_access(expr) {
                    return false;
                }
                self.expression_moves_non_copy_self_field(expr)
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                if self.expression_is_self_field_access(expr) {
                    return false;
                }
                self.expression_moves_non_copy_self_field(expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_moves_non_copy_self_field(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_moves_non_copy_self_field(s))
                    })
            }
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_moves_non_copy_self_field(s)),
            Statement::While { body, .. } => body
                .iter()
                .any(|s| self.statement_moves_non_copy_self_field(s)),
            Statement::Match { arms, .. } => arms
                .iter()
                .any(|arm| self.expression_moves_non_copy_self_field(arm.body)),
            _ => false,
        }
    }

    fn expression_moves_non_copy_self_field(&self, expr: &Expression) -> bool {
        match expr {
            // `self.field` or `self.a.b` used as a value (not in a method call position)
            Expression::FieldAccess { object, field, .. } => {
                if self.expression_is_self(object) {
                    let field_type = self.lookup_field_type_for_self(field);
                    if let Some(ft) = field_type {
                        return !self.is_copy_type(&ft);
                    }
                    return false;
                }
                // Nested field chain (e.g., self.graph.passes): if the parent
                // accesses a non-Copy self field, moving any sub-field also
                // requires owning self.
                self.expression_moves_non_copy_self_field(object)
            }
            // Struct literal: Foo { field: self.field, ... }
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_moves_non_copy_self_field(v)),
            // Block expression
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_moves_non_copy_self_field(s)),
            _ => false,
        }
    }

    fn expression_is_self(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Identifier { name, .. } if name == "self")
    }

    /// Look up the type of a field on `self` using the struct definition from the program.
    fn lookup_field_type_for_self(&self, field: &str) -> Option<Type> {
        let ctx = self.self_impl_context.as_ref()?;
        let program = ctx.program();
        let struct_name = &ctx.impl_type_base;

        for item in &program.items {
            if let crate::parser::Item::Struct { decl, .. } = item {
                if decl.name == *struct_name {
                    for sf in &decl.fields {
                        if sf.name == field {
                            return Some(sf.field_type.clone());
                        }
                    }
                }
            }
        }
        None
    }

    fn expression_returns_self_type(&self, expr: &Expression, type_name: &str) -> bool {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::StructLiteral { name, .. } if name == type_name => true,
            _ => false,
        }
    }

    /// Check if a function uses a specific identifier (e.g., "self")
    pub(super) fn function_uses_identifier(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            if self.statement_uses_identifier(name, stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement uses a specific identifier
    fn statement_uses_identifier(&self, name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => self.expression_uses_identifier(name, expr),
            Statement::Let { value, .. } => self.expression_uses_identifier(name, value),
            Statement::Assignment { target, value, .. } => {
                self.expression_uses_identifier(name, target)
                    || self.expression_uses_identifier(name, value)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_uses_identifier(name, expr),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_uses_identifier(name, condition)
                    || self.function_uses_identifier(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|block| self.function_uses_identifier(name, block))
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_uses_identifier(name, condition)
                    || self.function_uses_identifier(name, body)
            }
            Statement::For { iterable, body, .. } => {
                self.expression_uses_identifier(name, iterable)
                    || self.function_uses_identifier(name, body)
            }
            Statement::Match { value, arms, .. } => {
                self.expression_uses_identifier(name, value)
                    || arms
                        .iter()
                        .any(|arm| self.expression_uses_identifier(name, arm.body))
            }
            _ => false,
        }
    }

    /// Check if an expression uses a specific identifier
    pub(super) fn expression_uses_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::FieldAccess { object, .. } => self.expression_uses_identifier(name, object),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_uses_identifier(name, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_uses_identifier(name, arg))
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_uses_identifier(name, arg)),
            Expression::Binary { left, right, .. } => {
                self.expression_uses_identifier(name, left)
                    || self.expression_uses_identifier(name, right)
            }
            Expression::Unary { operand, .. } => self.expression_uses_identifier(name, operand),
            Expression::Index { object, index, .. } => {
                self.expression_uses_identifier(name, object)
                    || self.expression_uses_identifier(name, index)
            }
            Expression::Block { statements, .. } => self.function_uses_identifier(name, statements),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|el| self.expression_uses_identifier(name, el)),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|el| self.expression_uses_identifier(name, el)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                self.expression_uses_identifier(name, k) || self.expression_uses_identifier(name, v)
            }),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_uses_identifier(name, v)),
            Expression::Cast { expr, .. } => self.expression_uses_identifier(name, expr),
            Expression::Range { start, end, .. } => {
                self.expression_uses_identifier(name, start)
                    || self.expression_uses_identifier(name, end)
            }
            Expression::TryOp { expr, .. } => self.expression_uses_identifier(name, expr),
            _ => false,
        }
    }

    /// Check if a function accesses self fields (for impl methods)
    #[allow(dead_code)]
    pub(super) fn function_accesses_self_fields(&self, func: &FunctionDecl) -> bool {
        for stmt in &func.body {
            if self.statement_accesses_self_fields(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement accesses self fields
    fn statement_accesses_self_fields(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => self.expression_accesses_self_fields(expr),
            Statement::Let { value, .. } => self.expression_accesses_self_fields(value),
            Statement::Assignment { target, value, .. } => {
                self.expression_accesses_self_fields(target)
                    || self.expression_accesses_self_fields(value)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_accesses_self_fields(condition)
                    || then_block
                        .iter()
                        .any(|s| self.statement_accesses_self_fields(s))
                    || else_block.as_ref().is_some_and(|block| {
                        block.iter().any(|s| self.statement_accesses_self_fields(s))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_accesses_self_fields(condition)
                    || body.iter().any(|s| self.statement_accesses_self_fields(s))
            }
            Statement::For { iterable, body, .. } => {
                self.expression_accesses_self_fields(iterable)
                    || body.iter().any(|s| self.statement_accesses_self_fields(s))
            }
            _ => false,
        }
    }

    /// Check if expression accesses self fields
    #[allow(clippy::only_used_in_recursion)]
    fn expression_accesses_self_fields(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_accesses_self_fields(object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_accesses_self_fields(arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expression_accesses_self_fields(left)
                    || self.expression_accesses_self_fields(right)
            }
            Expression::Unary { operand, .. } => self.expression_accesses_self_fields(operand),
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expression_accesses_self_fields(arg)),
            Expression::MacroInvocation { args, .. } => args
                .iter()
                .any(|arg| self.expression_accesses_self_fields(arg)),
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|elem| self.expression_accesses_self_fields(elem)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|elem| self.expression_accesses_self_fields(elem)),
            _ => false,
        }
    }

    /// `match self.opt { Some(x) => x.foo() }` where `foo` takes `&mut self` requires `&mut self` on the outer method.
    fn function_mutates_through_self_option_scrutinee(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        func.body
            .iter()
            .any(|s| self.statement_mutates_through_self_option_scrutinee(s, registry))
    }

    fn statement_mutates_through_self_option_scrutinee(
        &self,
        stmt: &Statement,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match stmt {
            Statement::Match { value, arms, .. } => {
                self.expression_is_self_field_access(value)
                    && arms
                        .iter()
                        .any(|arm| self.match_arm_some_calls_mut_method_on_binding(arm, registry))
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_mutates_through_self_option_scrutinee(s, registry))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| {
                            self.statement_mutates_through_self_option_scrutinee(s, registry)
                        })
                    })
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_mutates_through_self_option_scrutinee(s, registry)),
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_mutates_through_self_option_scrutinee(s, registry)),
            _ => false,
        }
    }

    fn match_arm_some_calls_mut_method_on_binding(
        &self,
        arm: &MatchArm,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        let (variant, binding) = match &arm.pattern {
            Pattern::EnumVariant(v, EnumPatternBinding::Single(name)) => {
                (v.as_str(), name.as_str())
            }
            _ => return false,
        };
        let is_some_arm = variant == "Some" || variant.ends_with("::Some");
        if !is_some_arm {
            return false;
        }
        self.expr_calls_mut_self_method_on_identifier(arm.body, binding, registry)
    }

    fn expr_calls_mut_self_method_on_identifier(
        &self,
        expr: &Expression,
        id: &str,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        match expr {
            Expression::Block { statements, .. } => {
                self.block_expr_calls_mut_self_on_id(statements.as_slice(), id, registry)
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == id {
                        let mut_via_sig = registry.is_some_and(|reg| {
                            reg.get_signature(method).is_some_and(|sig| {
                                sig.has_self_receiver
                                    && sig.param_ownership.first()
                                        == Some(&super::OwnershipMode::MutBorrowed)
                            })
                        });
                        if mut_via_sig || self.is_mutating_method(method) {
                            return true;
                        }
                    }
                }
                self.expr_calls_mut_self_method_on_identifier(object, id, registry)
                    || arguments.iter().any(|(_, a)| {
                        self.expr_calls_mut_self_method_on_identifier(a, id, registry)
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.expr_calls_mut_self_method_on_identifier(left, id, registry)
                    || self.expr_calls_mut_self_method_on_identifier(right, id, registry)
            }
            Expression::Unary { operand, .. } => {
                self.expr_calls_mut_self_method_on_identifier(operand, id, registry)
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, a)| self.expr_calls_mut_self_method_on_identifier(a, id, registry)),
            _ => false,
        }
    }

    fn block_expr_calls_mut_self_on_id<'s>(
        &self,
        block: &[&'s Statement<'s>],
        id: &str,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        for s in block {
            match *s {
                Statement::Expression { expr, .. } => {
                    if self.expr_calls_mut_self_method_on_identifier(expr, id, registry) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expr_calls_mut_self_method_on_identifier(expr, id, registry) {
                        return true;
                    }
                }
                Statement::Let { value, .. } => {
                    if self.expr_calls_mut_self_method_on_identifier(value, id, registry) {
                        return true;
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    if self.block_expr_calls_mut_self_on_id(body, id, registry) {
                        return true;
                    }
                }
                Statement::For { body, .. } => {
                    if self.block_expr_calls_mut_self_on_id(body, id, registry) {
                        return true;
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expr_calls_mut_self_method_on_identifier(condition, id, registry) {
                        return true;
                    }
                    if self.block_expr_calls_mut_self_on_id(then_block, id, registry) {
                        return true;
                    }
                    if let Some(e) = else_block {
                        if self.block_expr_calls_mut_self_on_id(e, id, registry) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if an expression references a variable
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn expression_references_variable(&self, var_name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == var_name,
            Expression::Binary { left, right, .. } => {
                self.expression_references_variable(var_name, left)
                    || self.expression_references_variable(var_name, right)
            }
            Expression::MethodCall { object, .. } | Expression::FieldAccess { object, .. } => {
                self.expression_references_variable(var_name, object)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expression_references_variable(var_name, function)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_references_variable(var_name, arg))
            }
            _ => false,
        }
    }

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
