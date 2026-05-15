//! Self field reads, Option-scrutinee paths, variable references.

use crate::parser::*;

use super::Analyzer;
impl<'ast> Analyzer<'ast> {
    pub(super) fn function_accesses_self_fields(&self, func: &FunctionDecl) -> bool {
        for stmt in &func.body {
            if self.statement_accesses_self_fields(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement accesses self fields
    pub(crate) fn statement_accesses_self_fields(&self, stmt: &Statement) -> bool {
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
    pub(crate) fn expression_accesses_self_fields(&self, expr: &Expression) -> bool {
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
    pub(crate) fn function_mutates_through_self_option_scrutinee(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        func.body
            .iter()
            .any(|s| self.statement_mutates_through_self_option_scrutinee(s, registry))
    }

    pub(crate) fn statement_mutates_through_self_option_scrutinee(
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

    pub(crate) fn match_arm_some_calls_mut_method_on_binding(
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

    pub(crate) fn expr_calls_mut_self_method_on_identifier(
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

    pub(crate) fn block_expr_calls_mut_self_on_id<'s>(
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

}
