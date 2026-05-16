//! Statement- and expression-level detection of mutations to `self` fields.
use std::collections::HashSet;

use crate::parser::*;

use super::Analyzer;
impl<'ast> Analyzer<'ast> {
    pub(crate) fn statement_modifies_self_fields(
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
    pub(crate) fn expression_contains_self_field_mutations(
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
    pub(crate) fn expression_is_self_field_index_access(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Index { object, .. } => {
                self.expression_is_self_field_access(object)
                    || self.expression_is_self_field_index_access(object)
            }
            _ => false,
        }
    }

    /// Check if expression mutates self fields
    pub(crate) fn expression_mutates_self_fields(
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

    pub(crate) fn expression_mutates_self_fields_inner(
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
}
