//! Scanning statements and expressions for mutating method calls on bindings
//! (used by Option match / `&` vs `&mut` scrutinee prefix logic).

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    fn statement_binding_mut_method_scan(&self, stmt: &Statement<'ast>, binding: &str) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                super::self_analysis::expression_references_variable_or_field(target, binding)
            }
            Statement::Expression { expr, .. } => {
                self.expr_binding_receives_mutating_method_call(expr, binding)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_binding_receives_mutating_method_call(condition, binding)
                    || then_block
                        .iter()
                        .any(|s| self.statement_binding_mut_method_scan(s, binding))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.statement_binding_mut_method_scan(s, binding))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_binding_receives_mutating_method_call(condition, binding)
                    || body
                        .iter()
                        .any(|s| self.statement_binding_mut_method_scan(s, binding))
            }
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_binding_mut_method_scan(s, binding)),
            Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_binding_mut_method_scan(s, binding)),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_binding_receives_mutating_method_call(expr, binding),
            Statement::Let { value, .. } => {
                self.expr_binding_receives_mutating_method_call(value, binding)
            }
            Statement::Match { value, arms, .. } => {
                self.expr_binding_receives_mutating_method_call(value, binding)
                    || arms.iter().any(|arm| {
                        self.expr_binding_receives_mutating_method_call(arm.body, binding)
                    })
            }
            _ => false,
        }
    }

    pub(in crate::codegen::rust) fn expr_binding_receives_mutating_method_call(
        &self,
        expr: &Expression<'ast>,
        binding: &str,
    ) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_binding_mut_method_scan(s, binding)),
            Expression::MethodCall { object, method, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == binding && self.codegen_method_likely_mutates_receiver(method) {
                        return true;
                    }
                }
                self.expr_binding_receives_mutating_method_call(object, binding)
            }
            Expression::Binary { left, right, .. } => {
                self.expr_binding_receives_mutating_method_call(left, binding)
                    || self.expr_binding_receives_mutating_method_call(right, binding)
            }
            Expression::Unary { operand, .. } => {
                self.expr_binding_receives_mutating_method_call(operand, binding)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    if let Expression::Identifier { name, .. } = &**object {
                        if name == binding && self.codegen_method_likely_mutates_receiver(field) {
                            return true;
                        }
                    }
                }
                self.expr_binding_receives_mutating_method_call(function, binding)
                    || arguments
                        .iter()
                        .any(|(_, a)| self.expr_binding_receives_mutating_method_call(a, binding))
            }
            _ => false,
        }
    }

    fn codegen_method_likely_mutates_receiver(&self, method: &str) -> bool {
        super::stdlib_method_traits::method_mutates_receiver(method)
    }

    /// Like `expr_binding_receives_mutating_method_call` but also consults
    /// the signature registry for user-defined methods on `binding_type`.
    pub(in crate::codegen::rust) fn binding_receives_mutating_call_with_sig_check(
        &self,
        expr: &Expression<'ast>,
        binding: &str,
        binding_type: &Type,
    ) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.stmt_binding_mut_call_with_sig(s, binding, binding_type)),
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == binding
                        && self.method_mutates_via_registry_or_sig(method, binding_type)
                    {
                        return true;
                    }
                }
                if arguments.iter().enumerate().any(|(i, (_, a))| {
                    matches!(a, Expression::Identifier { name, .. } if name == binding)
                        && self.method_call_argument_expects_mut_borrow(object, method, i)
                }) {
                    return true;
                }
                self.binding_receives_mutating_call_with_sig_check(object, binding, binding_type)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::FieldAccess { object, field, .. } = &**function {
                    if let Expression::Identifier { name, .. } = &**object {
                        if name == binding
                            && self.method_mutates_via_registry_or_sig(field, binding_type)
                        {
                            return true;
                        }
                    }
                }
                self.binding_receives_mutating_call_with_sig_check(function, binding, binding_type)
                    || arguments.iter().any(|(_, a)| {
                        self.binding_receives_mutating_call_with_sig_check(a, binding, binding_type)
                    })
            }
            Expression::Binary { left, right, .. } => {
                self.binding_receives_mutating_call_with_sig_check(left, binding, binding_type)
                    || self.binding_receives_mutating_call_with_sig_check(
                        right,
                        binding,
                        binding_type,
                    )
            }
            Expression::Unary { operand, .. } => {
                self.binding_receives_mutating_call_with_sig_check(operand, binding, binding_type)
            }
            _ => false,
        }
    }

    fn stmt_binding_mut_call_with_sig(
        &self,
        stmt: &Statement<'ast>,
        binding: &str,
        binding_type: &Type,
    ) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                super::self_analysis::expression_references_variable_or_field(target, binding)
            }
            Statement::Expression { expr, .. } => {
                self.binding_receives_mutating_call_with_sig_check(expr, binding, binding_type)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.binding_receives_mutating_call_with_sig_check(condition, binding, binding_type)
                    || then_block
                        .iter()
                        .any(|s| self.stmt_binding_mut_call_with_sig(s, binding, binding_type))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_binding_mut_call_with_sig(s, binding, binding_type))
                    })
            }
            Statement::Match { value, arms, .. } => {
                self.binding_receives_mutating_call_with_sig_check(value, binding, binding_type)
                    || arms.iter().any(|arm| {
                        self.binding_receives_mutating_call_with_sig_check(
                            arm.body,
                            binding,
                            binding_type,
                        )
                    })
            }
            Statement::Return { value, .. } => value
                .map(|v| {
                    self.binding_receives_mutating_call_with_sig_check(v, binding, binding_type)
                })
                .unwrap_or(false),
            _ => false,
        }
    }

    /// Whether argument `arg_idx` of `receiver.method(...)` expects `&mut` (e.g. `is_available(world, …)`).
    fn method_call_argument_expects_mut_borrow(
        &self,
        receiver: &Expression<'ast>,
        method: &str,
        arg_idx: usize,
    ) -> bool {
        let recv_type = self.infer_expression_type(receiver);
        let type_name = match recv_type.as_ref() {
            Some(Type::Custom(name)) => name.as_str(),
            _ => "",
        };
        let qualified = if type_name.is_empty() {
            method.to_string()
        } else {
            format!("{}::{}", type_name, method)
        };
        let call_arg_count = arg_idx + 1;
        let Some(sig) = self
            .get_signature_with_global(&qualified)
            .or_else(|| {
                if type_name.is_empty() {
                    None
                } else {
                    self.find_method_on_receiver_with_global(type_name, method, call_arg_count)
                }
            })
            .or_else(|| {
                self.find_signature_by_name_and_arg_count_with_global(method, call_arg_count)
            }) else {
            return false;
        };
        let param_idx = if sig.has_self_receiver {
            arg_idx + 1
        } else {
            arg_idx
        };
        sig.param_ownership
            .get(param_idx)
            .is_some_and(|o| *o == crate::analyzer::OwnershipMode::MutBorrowed)
    }

    /// Check if a method on the given type is known to mutate its receiver,
    /// using both the stdlib method registry and the signature registry.
    fn method_mutates_via_registry_or_sig(&self, method: &str, receiver_type: &Type) -> bool {
        if super::stdlib_method_traits::method_mutates_receiver(method) {
            return true;
        }
        let type_name = match receiver_type {
            Type::Custom(name) => name.as_str(),
            _ => return false,
        };
        let qualified = format!("{}::{}", type_name, method);
        if let Some(sig) = self.get_signature_with_global(&qualified) {
            if sig.has_self_receiver && !sig.param_ownership.is_empty() {
                return sig.param_ownership[0] == crate::analyzer::OwnershipMode::MutBorrowed;
            }
        }
        false
    }
}
