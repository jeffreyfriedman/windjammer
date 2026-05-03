//! Passthrough ownership inference for the analyzer.
//! Multi-pass inference that matches parameter ownership to callee signatures
//! when a parameter is simply passed through to another function.

use crate::parser::*;

use super::{Analyzer, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    /// MULTI-PASS: Infer ownership from pass-through calls using signature registry
    /// If param is ONLY passed to functions whose signatures are known, match their ownership
    pub(super) fn infer_passthrough_ownership(
        &self,
        param_name: &str,
        param_type: &Type,
        body: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
        current_func_name: &str,
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
        self.collect_passthrough_calls(param_name, body, &mut passthrough_calls);

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
            // Look up the callee signature: try exact name first, then Type::method patterns
            let sig = match registry.get_signature(func_name) {
                Some(s) => s,
                None => {
                    // Fallback: for method calls, the name might be just "method" but registered
                    // as "Type::method". Search for entries ending with "::method".
                    match registry.find_signature_ending_with(func_name) {
                        Some(s) => s,
                        None => continue, // Unknown callee, skip (don't abort)
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
            match inferred_mode {
                None => inferred_mode = Some(ownership),
                Some(existing_mode) => {
                    if existing_mode != ownership {
                        return Some(OwnershipMode::Owned);
                    }
                }
            }
        }

        inferred_mode
    }

    /// TDD: Infer ownership from method calls made ON the parameter
    /// E.g., `grid.set(42)` where `set(&mut self, ...)` → grid needs `&mut Grid`
    /// E.g., `grid.get(0)` where `get(&self, ...)` → grid needs `&Grid`
    fn infer_from_method_calls_on_param(
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
    fn collect_method_calls_on_param(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        results: &mut Vec<String>,
    ) {
        for stmt in body {
            self.collect_method_calls_from_stmt(param_name, stmt, results);
        }
    }

    fn collect_method_calls_from_stmt(
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

    fn collect_method_calls_from_expr(
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
    fn collect_passthrough_calls(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        results: &mut Vec<(String, usize, bool)>,
    ) {
        for stmt in body {
            self.collect_passthrough_from_stmt(param_name, stmt, results);
        }
    }

    fn collect_passthrough_from_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        results: &mut Vec<(String, usize, bool)>,
    ) {
        match stmt {
            Statement::Expression {
                expr: expression, ..
            } => {
                self.collect_passthrough_from_expr(param_name, expression, results);
            }
            Statement::Let { value, .. } => {
                self.collect_passthrough_from_expr(param_name, value, results);
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.collect_passthrough_from_expr(param_name, expr, results);
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.collect_passthrough_from_expr(param_name, condition, results);
                for stmt in then_block {
                    self.collect_passthrough_from_stmt(param_name, stmt, results);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.collect_passthrough_from_stmt(param_name, stmt, results);
                    }
                }
            }
            Statement::While {
                condition,
                body: while_body,
                ..
            } => {
                self.collect_passthrough_from_expr(param_name, condition, results);
                for stmt in while_body {
                    self.collect_passthrough_from_stmt(param_name, stmt, results);
                }
            }
            Statement::For {
                iterable,
                body: for_body,
                ..
            } => {
                self.collect_passthrough_from_expr(param_name, iterable, results);
                for stmt in for_body {
                    self.collect_passthrough_from_stmt(param_name, stmt, results);
                }
            }
            Statement::Loop { body, .. } => {
                for stmt in body {
                    self.collect_passthrough_from_stmt(param_name, stmt, results);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.collect_passthrough_from_expr(param_name, value, results);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        self.collect_passthrough_from_expr(param_name, guard, results);
                    }
                    self.collect_passthrough_from_expr(param_name, arm.body, results);
                }
            }
            Statement::Assignment { value, .. } => {
                self.collect_passthrough_from_expr(param_name, value, results);
            }
            _ => {}
        }
    }

    fn collect_passthrough_from_expr(
        &self,
        param_name: &str,
        expr: &Expression,
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
                self.collect_passthrough_from_expr(param_name, function, results);
                for (_name, arg) in arguments {
                    self.collect_passthrough_from_expr(param_name, arg, results);
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
                        if let Some(method_name) = self.extract_method_name(object, method) {
                            results.push((method_name, i, is_self_field_call));
                        }
                    }
                }
                self.collect_passthrough_from_expr(param_name, object, results);
                for (_, arg) in arguments {
                    self.collect_passthrough_from_expr(param_name, arg, results);
                }
            }
            Expression::TryOp { expr, .. } => {
                self.collect_passthrough_from_expr(param_name, expr, results);
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_passthrough_from_stmt(param_name, stmt, results);
                }
            }
            Expression::Unary { operand, .. } => {
                self.collect_passthrough_from_expr(param_name, operand, results);
            }
            Expression::Binary { left, right, .. } => {
                self.collect_passthrough_from_expr(param_name, left, results);
                self.collect_passthrough_from_expr(param_name, right, results);
            }
            Expression::Index { object, index, .. } => {
                self.collect_passthrough_from_expr(param_name, object, results);
                self.collect_passthrough_from_expr(param_name, index, results);
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_passthrough_from_expr(param_name, object, results);
            }
            Expression::Tuple { elements, .. } => {
                for e in elements {
                    self.collect_passthrough_from_expr(param_name, e, results);
                }
            }
            _ => {}
        }
    }

    pub(super) fn expr_is_identifier(&self, expr: &Expression, name: &str) -> bool {
        matches!(expr, Expression::Identifier { name: id, .. } if id == name)
    }

    fn extract_function_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { field, .. } => Some(field.clone()),
            _ => None,
        }
    }

    fn extract_method_name(&self, _object: &Expression, method: &str) -> Option<String> {
        Some(method.to_string())
    }
}
