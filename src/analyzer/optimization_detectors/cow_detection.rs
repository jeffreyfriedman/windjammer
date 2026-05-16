//! Cow (clone-on-write) opportunities for string-like parameters.

use crate::parser::*;

use super::super::{Analyzer, CowOptimization, CowReason};

impl<'ast> Analyzer<'ast> {
    pub(in crate::analyzer) fn detect_cow_opportunities(
        &self,
        func: &FunctionDecl,
    ) -> Vec<CowOptimization> {
        let mut optimizations = Vec::new();

        for param in &func.parameters {
            let is_string_like = matches!(param.type_, Type::String)
                || matches!(
                    param.type_,
                    Type::Reference(ref inner) if matches!(**inner, Type::String)
                );

            if !is_string_like {
                continue;
            }

            if let Some(reason) = self.analyze_conditional_modification(&param.name, &func.body) {
                optimizations.push(CowOptimization {
                    variable: param.name.clone(),
                    reason,
                });
            }
        }

        optimizations
    }

    pub(crate) fn analyze_conditional_modification(
        &self,
        var_name: &str,
        body: &[&'ast Statement<'ast>],
    ) -> Option<CowReason> {
        let mut has_read_only_path = false;
        let mut has_modifying_path = false;

        for stmt in body {
            match stmt {
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    let modified_in_then = self.is_variable_modified(var_name, then_block);
                    let modified_in_else = else_block
                        .as_ref()
                        .map(|block| self.is_variable_modified(var_name, block))
                        .unwrap_or(false);

                    if modified_in_then != modified_in_else {
                        has_read_only_path = true;
                        has_modifying_path = true;
                    } else if !modified_in_then {
                        has_read_only_path = true;
                    } else {
                        has_modifying_path = true;
                    }
                }

                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if self.expression_references_variable(var_name, arm.body) {
                            has_read_only_path = true;
                        }
                    }
                }

                Statement::Expression { expr, .. }
                | Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expression_references_variable(var_name, expr) {
                        has_read_only_path = true;
                    }
                }

                _ => {}
            }
        }

        if has_read_only_path && has_modifying_path {
            Some(CowReason::ConditionalModification)
        } else {
            None
        }
    }

    pub(crate) fn is_variable_modified(
        &self,
        var_name: &str,
        statements: &[&'ast Statement<'ast>],
    ) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Assignment {
                    target: Expression::Identifier { name, .. },
                    ..
                } if name == var_name => {
                    return true;
                }

                Statement::Expression {
                    expr: Expression::MethodCall { object, method, .. },
                    ..
                } => {
                    if let Expression::Identifier { name, .. } = object {
                        if name == var_name && self.is_mutating_method(method) {
                            return true;
                        }
                    }
                }

                _ => {}
            }
        }
        false
    }

    pub(in crate::analyzer) fn is_mutating_method(&self, method: &str) -> bool {
        crate::method_registry::mutates_receiver(method)
    }
}
