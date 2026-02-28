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
        body: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) -> Option<OwnershipMode> {
        let mut passthrough_calls = Vec::new();
        self.collect_passthrough_calls(param_name, body, &mut passthrough_calls);

        if passthrough_calls.is_empty() {
            return None;
        }
        
        let mut inferred_mode: Option<OwnershipMode> = None;
        
        for (func_name, arg_position) in &passthrough_calls {
            if let Some(sig) = registry.get_signature(func_name) {
                // Adjust position: method calls store natural arg index (0-based);
                // if the signature has an explicit self receiver, offset by 1
                let adjusted_position = if sig.has_self_receiver {
                    *arg_position + 1
                } else {
                    *arg_position
                };
                if let Some(&ownership) = sig.param_ownership.get(adjusted_position) {
                    match inferred_mode {
                        None => inferred_mode = Some(ownership),
                        Some(existing_mode) => {
                            // If different calls need different ownership, take most restrictive (Owned)
                            if existing_mode != ownership {
                                return Some(OwnershipMode::Owned);
                            }
                        }
                    }
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        
        inferred_mode
    }
    
    /// Helper: Collect all function calls where param is passed as an argument
    /// Returns (function_name, argument_position)
    fn collect_passthrough_calls(
        &self,
        param_name: &str,
        body: &[&'ast Statement<'ast>],
        results: &mut Vec<(String, usize)>,
    ) {
        for stmt in body {
            self.collect_passthrough_from_stmt(param_name, stmt, results);
        }
    }
    
    fn collect_passthrough_from_stmt(
        &self,
        param_name: &str,
        stmt: &Statement,
        results: &mut Vec<(String, usize)>,
    ) {
        match stmt {
            Statement::Expression { expr: expression, .. } => {
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
            Statement::If { condition, then_block, else_block, .. } => {
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
            Statement::While { condition, body: while_body, .. } => {
                self.collect_passthrough_from_expr(param_name, condition, results);
                for stmt in while_body {
                    self.collect_passthrough_from_stmt(param_name, stmt, results);
                }
            }
            Statement::For { iterable, body: for_body, .. } => {
                self.collect_passthrough_from_expr(param_name, iterable, results);
                for stmt in for_body {
                    self.collect_passthrough_from_stmt(param_name, stmt, results);
                }
            }
            _ => {}
        }
    }
    
    fn collect_passthrough_from_expr(
        &self,
        param_name: &str,
        expr: &Expression,
        results: &mut Vec<(String, usize)>,
    ) {
        match expr {
            Expression::Call { function, arguments, .. } => {
                for (i, (_name, arg)) in arguments.iter().enumerate() {
                    if self.expr_is_identifier(arg, param_name) {
                        if let Some(func_name) = self.extract_function_name(function) {
                            results.push((func_name, i));
                        }
                    }
                }
                self.collect_passthrough_from_expr(param_name, function, results);
                for (_name, arg) in arguments {
                    self.collect_passthrough_from_expr(param_name, arg, results);
                }
            }
            Expression::MethodCall { object, method, arguments, .. } => {
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if self.expr_is_identifier(arg, param_name) {
                        if let Some(method_name) = self.extract_method_name(object, method) {
                            // Store natural argument index; adjusted at lookup time
                            // based on whether the signature has an explicit self receiver
                            results.push((method_name, i));
                        }
                    }
                }
                self.collect_passthrough_from_expr(param_name, object, results);
                for (_, arg) in arguments {
                    self.collect_passthrough_from_expr(param_name, arg, results);
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
