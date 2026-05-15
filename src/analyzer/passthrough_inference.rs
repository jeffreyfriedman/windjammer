//! Passthrough ownership inference for the analyzer.
//! Multi-pass inference that matches parameter ownership to callee signatures
//! when a parameter is simply passed through to another function.

use crate::parser::*;

use super::{Analyzer, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    pub(crate) fn strip_type_generics(name: &str) -> String {
        name.split('<').next().unwrap_or(name).to_string()
    }

    /// Structural type name used as `SignatureRegistry` keys (`Inventory`, `Merchant`, …).
    pub(crate) fn type_to_struct_base(ty: &Type) -> Option<String> {
        match ty {
            Type::Custom(name) => Some(Self::strip_type_generics(name)),
            Type::Parameterized(base, _) => Some(Self::strip_type_generics(base)),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::type_to_struct_base(inner)
            }
            _ => None,
        }
    }

    /// Resolve the static type backing a method-call receiver (`self`, param, `self.field`, …).
    pub(crate) fn infer_receiver_type_base(&self, object: &Expression, func: &FunctionDecl<'ast>) -> Option<String> {
        match object {
            Expression::Identifier { name, .. } if name == "self" => func
                .parent_type
                .as_ref()
                .map(|p| Self::strip_type_generics(p)),
            Expression::Identifier { name, .. } => func
                .parameters
                .iter()
                .find(|p| &p.name == name)
                .and_then(|p| Self::type_to_struct_base(&p.type_)),
            Expression::FieldAccess { object: inner, field, .. } => {
                let inner_base = self.infer_receiver_type_base(inner, func)?;
                self.global_struct_field_types
                    .get(&inner_base)
                    .and_then(|m| m.get(field.as_str()))
                    .and_then(Self::type_to_struct_base)
            }
            Expression::Unary {
                op: UnaryOp::Ref | UnaryOp::MutRef,
                operand,
                ..
            } => self.infer_receiver_type_base(operand, func),
            _ => None,
        }
    }

    /// Static type for `self...` receivers inside an `impl`, for `Type::method` registry keys.
    pub(crate) fn static_value_type_of_self_rooted_expr(
        &self,
        _program: &Program<'ast>,
        impl_type_base: &str,
        expr: &Expression<'ast>,
    ) -> Option<Type> {
        match expr {
            Expression::Identifier { name, .. } if name == "self" => {
                Some(Type::Custom(impl_type_base.to_string()))
            }
            Expression::FieldAccess { object, field, .. } => {
                let inner_ty =
                    self.static_value_type_of_self_rooted_expr(_program, impl_type_base, object)?;
                let inner_base = Self::type_to_struct_base(&inner_ty)?;
                self.global_struct_field_types
                    .get(&inner_base)
                    .and_then(|m| m.get(field.as_str()))
                    .cloned()
            }
            Expression::Unary {
                op: UnaryOp::Ref | UnaryOp::MutRef,
                operand,
                ..
            } => self.static_value_type_of_self_rooted_expr(_program, impl_type_base, operand),
            _ => None,
        }
    }

    pub(crate) fn type_base_for_qualified_sig_lookup(ty: &Type) -> Option<String> {
        Self::type_to_struct_base(ty)
    }

    /// Registry lookup key matching [`SignatureRegistry`] (`Type::method`), not ambiguous `method` alone.
    pub(crate) fn qualified_method_registry_key(
        &self,
        object: &Expression,
        method: &str,
        func: &FunctionDecl<'ast>,
    ) -> String {
        self.infer_receiver_type_base(object, func)
            .map(|base| format!("{}::{}", base, method))
            .unwrap_or_else(|| method.to_string())
    }

    /// MULTI-PASS: Infer ownership from pass-through calls using signature registry
    /// If param is ONLY passed to functions whose signatures are known, match their ownership
    pub(super) fn infer_passthrough_ownership(
        &self,
        param_name: &str,
        param_type: &Type,
        body: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
        current_func_name: &str,
        func: &FunctionDecl<'ast>,
    ) -> Option<OwnershipMode> {
        // TDD: Check for METHOD CALLS ON the parameter first (e.g., grid.set(42))
        // This determines if parameter needs &mut based on method's self type
        //
        // THE WINDJAMMER WAY: Multi-pass compilation makes this work
        // - Pass 1: Grid::set isn't registered yet, fallback to other inference
        // - Pass 2: Grid::set is registered, we look it up and see it needs &mut self
        // - Result: fill_grid(grid: &mut Grid) correctly inferred!
        if let Some(method_self_mode) =
            self.infer_from_method_calls_on_param(param_name, body, registry)
        {
            return Some(method_self_mode);
        }

        // Then check for pass-through calls (parameter passed AS argument)
        // (func_name, arg_position, is_self_field_call)
        let mut passthrough_calls: Vec<(String, usize, bool)> = Vec::new();
        self.collect_passthrough_calls(param_name, body, func, &mut passthrough_calls);

        // Skip recursive calls to the current function to break circular ownership inference.
        // Without this, recursive functions like `traverse(bvh, ray)` calling `traverse(bvh, ray)`
        // would see their own Owned signature and keep inferring Owned, preventing convergence.
        // BUT: self.field.method() calls are on a DIFFERENT type even if the method name matches,
        // so don't filter those (e.g., Merchant::add_item calling self.inventory.add_item).
        passthrough_calls
            .retain(|(func_name, _, is_field)| *is_field || func_name != current_func_name);

        if passthrough_calls.is_empty() {
            return None;
        }

        let mut inferred_mode: Option<OwnershipMode> = None;

        for (func_name, arg_position, _is_field) in &passthrough_calls {
            // Look up the callee signature with multiple fallback strategies:
            // 1. Exact name (e.g., "place_marker" or "StationBuilder::place_marker")
            // 2. Suffix match (e.g., find "Type::method" from "method")
            // 3. Simple name from qualified (e.g., "place_marker" from "station_builder::place_marker")
            //    This handles cross-crate calls where metadata stores the simple name
            //    but the call site uses the module-qualified name.
            let sig = match registry.get_signature(func_name) {
                Some(s) => s,
                None => {
                    match registry.find_signature_ending_with(func_name) {
                        Some(s) => s,
                        None => {
                            if let Some(simple) = func_name.rsplit("::").next() {
                                if simple != func_name {
                                    match registry.get_signature(simple) {
                                        Some(s) => s,
                                        None => continue,
                                    }
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }
                    }
                }
            };
            let adjusted_position = if sig.has_self_receiver {
                *arg_position + 1
            } else {
                *arg_position
            };
            if adjusted_position >= sig.param_ownership.len() {
                continue;
            }
            if let Some(expected_ty) = sig.param_types.get(adjusted_position) {
                if !self.passthrough_types_compatible(expected_ty, param_type) {
                    continue;
                }
            }
            let Some(&ownership) = sig.param_ownership.get(adjusted_position) else {
                continue;
            };
            // TDD FIX: Use the STRONGEST ownership mode, not Owned on conflict.
            // In Rust, &mut T can always be reborrowed as &T, so:
            //   MutBorrowed + Borrowed → MutBorrowed (caller provides &mut, callees reborrow as needed)
            //   MutBorrowed + Owned → Owned (one callee consumes it)
            //   Borrowed + Owned → Owned (one callee consumes it)
            // The old code returned Owned whenever any two modes disagreed, which broke
            // the common pattern of passing a &mut parameter to both mutating and read-only functions.
            inferred_mode = Some(match (inferred_mode, ownership) {
                (None, mode) => mode,
                (Some(OwnershipMode::Owned), _) | (_, OwnershipMode::Owned) => OwnershipMode::Owned,
                (Some(OwnershipMode::MutBorrowed), _) | (_, OwnershipMode::MutBorrowed) => {
                    OwnershipMode::MutBorrowed
                }
                _ => OwnershipMode::Borrowed,
            });
        }

        inferred_mode
    }

    /// TDD: Infer ownership from method calls made ON the parameter
    /// E.g., `grid.set(42)` where `set(&mut self, ...)` → grid needs `&mut Grid`
    /// E.g., `grid.get(0)` where `get(&self, ...)` → grid needs `&Grid`
    pub(crate) fn infer_from_method_calls_on_param(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) -> Option<OwnershipMode> {
        let mut method_calls = Vec::new();
        self.collect_method_calls_on_param(param_name, body, &mut method_calls);

        if method_calls.is_empty() {
            return None;
        }

        let mut max_mode: Option<OwnershipMode> = None;

        for method_name in &method_calls {
            let sig = registry
                .get_signature(method_name)
                .or_else(|| registry.find_signature_ending_with(method_name));
            if let Some(sig) = sig {
                if let Some(&self_ownership) = sig.param_ownership.first() {
                    max_mode = Some(match max_mode {
                        None => self_ownership,
                        Some(current) => match (current, self_ownership) {
                            (OwnershipMode::Owned, _) | (_, OwnershipMode::Owned) => {
                                OwnershipMode::Owned
                            }
                            (OwnershipMode::MutBorrowed, _) | (_, OwnershipMode::MutBorrowed) => {
                                OwnershipMode::MutBorrowed
                            }
                            _ => OwnershipMode::Borrowed,
                        },
                    });
                }
            }
        }

        max_mode
    }

    /// Collect method calls made ON the parameter (param is the receiver)
    /// E.g., `grid.set(42)` → collect "set"
    pub(crate) fn collect_method_calls_on_param(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        results: &mut Vec<String>,
    ) {
        for stmt in body {
            self.collect_method_calls_from_stmt(param_name, stmt, results);
        }
    }

    pub(crate) fn collect_method_calls_from_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        results: &mut Vec<String>,
    ) {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.collect_method_calls_from_expr(param_name, expr, results);
            }
            Statement::Let { value, .. } => {
                self.collect_method_calls_from_expr(param_name, value, results);
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.collect_method_calls_from_expr(param_name, expr, results);
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_method_calls_from_expr(param_name, condition, results);
                for stmt in then_block {
                    self.collect_method_calls_from_stmt(param_name, stmt, results);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.collect_method_calls_from_stmt(param_name, stmt, results);
                    }
                }
            }
            Statement::While {
                condition,
                body: while_body,
                ..
            } => {
                self.collect_method_calls_from_expr(param_name, condition, results);
                for stmt in while_body {
                    self.collect_method_calls_from_stmt(param_name, stmt, results);
                }
            }
            Statement::For {
                iterable,
                body: for_body,
                ..
            } => {
                self.collect_method_calls_from_expr(param_name, iterable, results);
                for stmt in for_body {
                    self.collect_method_calls_from_stmt(param_name, stmt, results);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn collect_method_calls_from_expr(
        &self,
        param_name: &str,
        expr: &Expression,
        results: &mut Vec<String>,
    ) {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Check if method is called ON the parameter
                if self.expr_is_identifier(object, param_name) {
                    results.push(method.clone());
                }
                // Recurse into nested expressions
                self.collect_method_calls_from_expr(param_name, object, results);
                for (_, arg) in arguments {
                    self.collect_method_calls_from_expr(param_name, arg, results);
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.collect_method_calls_from_expr(param_name, function, results);
                for (_, arg) in arguments {
                    self.collect_method_calls_from_expr(param_name, arg, results);
                }
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_method_calls_from_expr(param_name, object, results);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_method_calls_from_expr(param_name, left, results);
                self.collect_method_calls_from_expr(param_name, right, results);
            }
            Expression::Unary { operand, .. } => {
                self.collect_method_calls_from_expr(param_name, operand, results);
            }
            // TDD FIX: Recurse into TryOp (?) expressions
            // Example: loader.load(...)? wraps the method call in TryOp
            Expression::TryOp { expr, .. } => {
                self.collect_method_calls_from_expr(param_name, expr, results);
            }
            _ => {}
        }
    }

    /// Helper: Collect all function calls where param is passed as an argument
    /// Returns (function_name, argument_position, is_self_field_call)
    pub(crate) fn collect_passthrough_calls(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        func: &FunctionDecl<'ast>,
        results: &mut Vec<(String, usize, bool)>,
    ) {
        for stmt in body {
            self.collect_passthrough_from_stmt(param_name, stmt, func, results);
        }
    }

    pub(crate) fn collect_passthrough_from_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        func: &FunctionDecl<'ast>,
        results: &mut Vec<(String, usize, bool)>,
    ) {
        match stmt {
            Statement::Expression {
                expr: expression, ..
            } => {
                self.collect_passthrough_from_expr(param_name, expression, func, results);
            }
            Statement::Let { value, .. } => {
                self.collect_passthrough_from_expr(param_name, value, func, results);
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.collect_passthrough_from_expr(param_name, expr, func, results);
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_passthrough_from_expr(param_name, condition, func, results);
                for stmt in then_block {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                    }
                }
            }
            Statement::While {
                condition,
                body: while_body,
                ..
            } => {
                self.collect_passthrough_from_expr(param_name, condition, func, results);
                for stmt in while_body {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
            }
            Statement::For {
                iterable,
                body: for_body,
                ..
            } => {
                self.collect_passthrough_from_expr(param_name, iterable, func, results);
                for stmt in for_body {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
            }
            Statement::Loop { body, .. } => {
                for stmt in body {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.collect_passthrough_from_expr(param_name, value, func, results);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        self.collect_passthrough_from_expr(param_name, guard, func, results);
                    }
                    self.collect_passthrough_from_expr(param_name, arm.body, func, results);
                }
            }
            Statement::Assignment { value, .. } => {
                self.collect_passthrough_from_expr(param_name, value, func, results);
            }
            _ => {}
        }
    }

    pub(crate) fn collect_passthrough_from_expr(
        &self,
        param_name: &str,
        expr: &Expression,
        func: &FunctionDecl<'ast>,
        results: &mut Vec<(String, usize, bool)>,
    ) {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                for (i, (_name, arg)) in arguments.iter().enumerate() {
                    if self.expr_is_identifier(arg, param_name) {
                        if let Some(func_name) = self.extract_function_name(function) {
                            results.push((func_name, i, false));
                        }
                    }
                }
                self.collect_passthrough_from_expr(param_name, function, func, results);
                for (_name, arg) in arguments {
                    self.collect_passthrough_from_expr(param_name, arg, func, results);
                }
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let is_self_field_call = matches!(&**object, Expression::FieldAccess { object: inner, .. }
                    if matches!(&**inner, Expression::Identifier { name, .. } if name == "self"));
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if self.expr_is_identifier(arg, param_name) {
                        let method_key = self.qualified_method_registry_key(object, method, func);
                        results.push((method_key, i, is_self_field_call));
                    }
                }
                self.collect_passthrough_from_expr(param_name, object, func, results);
                for (_, arg) in arguments {
                    self.collect_passthrough_from_expr(param_name, arg, func, results);
                }
            }
            Expression::TryOp { expr, .. } => {
                self.collect_passthrough_from_expr(param_name, expr, func, results);
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_passthrough_from_stmt(param_name, stmt, func, results);
                }
            }
            Expression::Unary { operand, .. } => {
                self.collect_passthrough_from_expr(param_name, operand, func, results);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_passthrough_from_expr(param_name, left, func, results);
                self.collect_passthrough_from_expr(param_name, right, func, results);
            }
            Expression::Index { object, index, .. } => {
                self.collect_passthrough_from_expr(param_name, object, func, results);
                self.collect_passthrough_from_expr(param_name, index, func, results);
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_passthrough_from_expr(param_name, object, func, results);
            }
            Expression::Tuple { elements, .. } => {
                for e in elements {
                    self.collect_passthrough_from_expr(param_name, e, func, results);
                }
            }
            _ => {}
        }
    }

    pub(super) fn expr_is_identifier(&self, expr: &Expression, name: &str) -> bool {
        matches!(expr, Expression::Identifier { name: id, .. } if id == name)
    }

    pub(crate) fn extract_function_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { field, .. } => Some(field.clone()),
            _ => None,
        }
    }
}
