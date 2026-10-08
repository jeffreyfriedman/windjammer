//! Call-site ownership signals: consuming methods/receivers and by-value argument passing.
//!
//! Uses [`SignatureRegistry`](super::SignatureRegistry) to align with callee `self` and parameter modes.

use crate::parser::*;

use super::{Analyzer, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    /// TDD: Check if a parameter calls a method that takes `self` by value (consuming).
    /// When `a.as_float()` is called and `as_float` takes owned `self`, `a` is consumed.
    /// This requires looking up the method's signature in the registry.
    pub(crate) fn calls_consuming_method(
        &self,
        param_name: &str,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
        receiver_type: Option<&str>,
    ) -> bool {
        for stmt in statements {
            if self.stmt_calls_consuming_method(param_name, stmt, registry, receiver_type) {
                return true;
            }
        }
        false
    }

    pub(crate) fn stmt_calls_consuming_method(
        &self,
        param_name: &str,
        stmt: &Statement<'ast>,
        registry: &SignatureRegistry,
        receiver_type: Option<&str>,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expr_calls_consuming_method(param_name, expr, registry, receiver_type)
            }
            Statement::Let { value, .. } => {
                self.expr_calls_consuming_method(param_name, value, registry, receiver_type)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_calls_consuming_method(param_name, expr, registry, receiver_type),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                if self.expr_calls_consuming_method(param_name, condition, registry, receiver_type)
                {
                    return true;
                }
                if self.calls_consuming_method(param_name, then_block, registry, receiver_type) {
                    return true;
                }
                if let Some(else_b) = else_block {
                    if self.calls_consuming_method(param_name, else_b, registry, receiver_type) {
                        return true;
                    }
                }
                false
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_calls_consuming_method(param_name, condition, registry, receiver_type)
                    || self.calls_consuming_method(param_name, body, registry, receiver_type)
            }
            Statement::Loop { body, .. } => {
                self.calls_consuming_method(param_name, body, registry, receiver_type)
            }
            Statement::For { body, .. } => {
                self.calls_consuming_method(param_name, body, registry, receiver_type)
            }
            Statement::Match { value, arms, .. } => {
                if self.expr_calls_consuming_method(param_name, value, registry, receiver_type) {
                    return true;
                }
                for arm in arms {
                    if self.expr_calls_consuming_method(
                        param_name,
                        arm.body,
                        registry,
                        receiver_type,
                    ) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn method_consumes_this_receiver(
        method: &str,
        receiver_type: Option<&str>,
        registry: &SignatureRegistry,
    ) -> bool {
        let owned_self = |sig: &super::FunctionSignature| {
            sig.has_self_receiver
                && matches!(sig.param_ownership.first(), Some(OwnershipMode::Owned))
        };
        if let Some(sig) =
            super::stdlib_method_traits::lookup_method_signature(method, receiver_type, registry)
        {
            return owned_self(sig);
        }
        // Bare `get_signature("as_float")` misses `Val::as_float` and can hit a
        // different type. Only an unambiguous unqualified key may decide.
        if registry.has_method_name_collision(method) {
            return false;
        }
        registry.get_signature(method).is_some_and(owned_self)
    }

    pub(crate) fn expr_calls_consuming_method(
        &self,
        param_name: &str,
        expr: &Expression<'ast>,
        registry: &SignatureRegistry,
        receiver_type: Option<&str>,
    ) -> bool {
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Direct receiver: `Val::as_float`, not a colliding bare `as_float`.
                if self.is_direct_receiver(param_name, object)
                    && Self::method_consumes_this_receiver(method, receiver_type, registry)
                {
                    return true;
                }
                // Check if param is passed as an owned argument to the method
                if let Some(sig) =
                    super::stdlib_method_traits::lookup_method_signature(method, None, registry)
                        .or_else(|| {
                            if registry.has_method_name_collision(method) {
                                None
                            } else {
                                registry.get_signature(method)
                            }
                        })
                {
                    let param_offset = if sig.has_self_receiver { 1 } else { 0 };
                    for (i, (_, arg)) in arguments.iter().enumerate() {
                        if matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                        {
                            let sig_idx = i + param_offset;
                            if let Some(mode) = sig.param_ownership.get(sig_idx) {
                                if matches!(mode, OwnershipMode::Owned) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                // Recurse into arguments and object
                if self.expr_calls_consuming_method(param_name, object, registry, receiver_type) {
                    return true;
                }
                for (_, arg) in arguments {
                    if self.expr_calls_consuming_method(param_name, arg, registry, receiver_type) {
                        return true;
                    }
                }
                false
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Handle Call(FieldAccess) pattern: param.method(args) where the
                // receiver is `param` and the method takes owned `self`.
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    if self.is_direct_receiver(param_name, object)
                        && Self::method_consumes_this_receiver(field, receiver_type, registry)
                    {
                        return true;
                    }
                }
                // Do NOT treat "passed to a free/associated function whose formal is
                // currently Owned" as a consuming method. That duplicated passthrough,
                // and mutual recursion (`tick_node` ↔ `tick_seq`) kept both Owned forever
                // — forcing `tree.clone()` on read-only BT walks (WDB-366). True consumes
                // are covered by is_stored / is_returned / passthrough strength.
                for (_, arg) in arguments {
                    if self.expr_calls_consuming_method(param_name, arg, registry, receiver_type) {
                        return true;
                    }
                }
                false
            }
            Expression::Block { statements, .. } => {
                self.calls_consuming_method(param_name, statements, registry, receiver_type)
            }
            Expression::Binary { left, right, .. } => {
                self.expr_calls_consuming_method(param_name, left, registry, receiver_type)
                    || self.expr_calls_consuming_method(param_name, right, registry, receiver_type)
            }
            Expression::Unary { operand, .. } => {
                self.expr_calls_consuming_method(param_name, operand, registry, receiver_type)
            }
            Expression::TryOp { expr, .. } => {
                self.expr_calls_consuming_method(param_name, expr, registry, receiver_type)
            }
            _ => false,
        }
    }

    /// Check if a parameter is the direct receiver of a method call.
    pub(crate) fn is_direct_receiver(&self, param_name: &str, object: &Expression) -> bool {
        match object {
            Expression::Identifier { name, .. } => name == param_name,
            _ => false,
        }
    }

    /// Check if a parameter is passed as a direct (non-&) argument to a function or method call.
    /// When a parameter is passed directly (not via &) to another function, it could be consumed
    /// (the callee may take ownership). Without knowing the callee's signature, we conservatively
    /// assume consumption and keep the parameter Owned.
    ///
    /// Examples that trigger Owned:
    /// - `Quest::new(id, title, description)` — id is a direct argument
    /// - `process(data)` — data is a direct argument
    ///
    /// Examples that do NOT trigger Owned:
    /// - `data.len()` — data is the receiver, not an argument
    /// - `process(&data)` — & wraps the argument, so it's borrowed
    /// - `format!("{}", data)` — macro call, not a function call in the AST
    #[allow(dead_code)]
    pub(crate) fn is_passed_as_argument(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            if self.stmt_passes_as_argument(name, stmt) {
                return true;
            }
        }
        false
    }

    pub(crate) fn stmt_passes_as_argument(&self, name: &str, stmt: &Statement<'ast>) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expr_passes_as_argument(name, value),
            Statement::Expression { expr, .. } => self.expr_passes_as_argument(name, expr),
            Statement::Return { value: Some(v), .. } => self.expr_passes_as_argument(name, v),
            Statement::Assignment { value, .. } => self.expr_passes_as_argument(name, value),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_passes_as_argument(name, condition)
                    || self.is_passed_as_argument(name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| self.is_passed_as_argument(name, b))
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_passes_as_argument(name, condition)
                    || self.is_passed_as_argument(name, body)
            }
            Statement::Loop { body, .. } => self.is_passed_as_argument(name, body),
            Statement::For { body, .. } => self.is_passed_as_argument(name, body),
            Statement::Match { value, arms, .. } => {
                self.expr_passes_as_argument(name, value)
                    || arms
                        .iter()
                        .any(|arm| self.expr_passes_as_argument(name, arm.body))
            }
            _ => false,
        }
    }

    pub(crate) fn expr_passes_as_argument(&self, name: &str, expr: &Expression<'ast>) -> bool {
        match expr {
            // Function call: check if parameter is a bare argument (not wrapped in &)
            Expression::Call { arguments, .. } => {
                // TDD FIX: Don't force Owned for simple pass-through!
                // If a parameter is ONLY passed to another function with no other operations,
                // it might be a pass-through and can stay Borrowed.
                //
                // CONSERVATIVE APPROACH: Still return true (force Owned) because without
                // the callee's signature (which doesn't exist during analysis), we can't
                // know if the callee consumes the value or just borrows it.
                //
                // FUTURE: Multi-pass analysis could solve this:
                // - Pass 1: Conservative inference
                // - Pass 2: Re-infer using SignatureRegistry from Pass 1
                // - Iterate until stable
                for (_label, arg) in arguments {
                    // Direct identifier: `f(param)` → potentially consuming
                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                        return true;
                    }
                    // Recursively check sub-expressions for nested calls
                    if self.expr_passes_as_argument(name, arg) {
                        return true;
                    }
                }
                false
            }
            // Method call: check arguments (NOT the receiver)
            Expression::MethodCall {
                object, arguments, ..
            } => {
                for (_label, arg) in arguments {
                    // Direct identifier: `obj.method(param)` → consuming
                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                        return true;
                    }
                    // Recursively check sub-expressions
                    if self.expr_passes_as_argument(name, arg) {
                        return true;
                    }
                }
                // Also check the receiver for nested calls (but NOT as a direct argument)
                self.expr_passes_as_argument(name, object)
            }
            // Block expression: check all statements
            Expression::Block { statements, .. } => self.is_passed_as_argument(name, statements),
            // Binary, unary, index, etc.: recurse into sub-expressions
            Expression::Binary { left, right, .. } => {
                self.expr_passes_as_argument(name, left)
                    || self.expr_passes_as_argument(name, right)
            }
            Expression::Unary { operand, .. } => self.expr_passes_as_argument(name, operand),
            Expression::Index { object, index, .. } => {
                self.expr_passes_as_argument(name, object)
                    || self.expr_passes_as_argument(name, index)
            }
            Expression::FieldAccess { object, .. } => self.expr_passes_as_argument(name, object),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| self.expr_passes_as_argument(name, e)),
            Expression::Closure { body, .. } => self.expr_passes_as_argument(name, body),
            // TDD FIX: TryOp wraps expressions with `?` (error propagation).
            // e.g., `process(data)?` produces TryOp { expr: Call { args: [data] } }
            // We must recurse into the inner expression to detect argument passing.
            Expression::TryOp { expr, .. } => self.expr_passes_as_argument(name, expr),
            // Note: We do NOT check Expression::Identifier here because bare identifiers
            // outside of Call/MethodCall arguments are not consuming (e.g., `data.len()`)
            _ => false,
        }
    }
}
