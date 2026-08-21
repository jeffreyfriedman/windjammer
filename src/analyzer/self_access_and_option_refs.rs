//! Self field reads, Option-scrutinee paths, variable references.

use crate::parser::*;

use super::Analyzer;

#[allow(dead_code)] // Entry points retained for Option/self analysis experiments
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
                    && arms.iter().any(|arm| {
                        self.match_arm_some_calls_mut_method_on_binding(value, arm, registry)
                    })
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expression_passes_self_field_to_mut_method_arg(condition, registry)
                    || then_block
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

    /// `if choice.is_available(world, player)` where `world` came from `if let Some(world) = self.world`.
    pub(crate) fn expression_passes_self_field_to_mut_method_arg(
        &self,
        expr: &Expression,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        let Some(reg) = registry else {
            return false;
        };
        match expr {
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => arguments.iter().enumerate().any(|(i, (_, a))| {
                self.expression_is_self_field_access(a)
                    && self.method_call_argument_expects_mut_borrow(object, method, i, reg)
            }),
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Some(func_name) = self.extract_function_name(function) {
                    let impl_ty = self
                        .self_impl_context
                        .as_ref()
                        .map(|c| c.impl_type_base.as_str());
                    return arguments.iter().enumerate().any(|(i, (_, a))| {
                        self.expression_is_self_field_access(a)
                            && super::stdlib_method_traits::callable_arg_expects_mut_borrow(
                                &func_name, impl_ty, i, true, reg,
                            )
                    });
                }
                false
            }
            Expression::Binary { left, right, .. } => {
                self.expression_passes_self_field_to_mut_method_arg(left, registry)
                    || self.expression_passes_self_field_to_mut_method_arg(right, registry)
            }
            _ => false,
        }
    }

    pub(crate) fn match_arm_some_calls_mut_method_on_binding(
        &self,
        scrutinee: &Expression,
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
        let binding_types = self.infer_match_arm_binding_type_bases(scrutinee, &arm.pattern);
        self.expr_calls_mut_self_method_on_identifier(
            arm.body,
            binding,
            registry,
            Some(&binding_types),
        ) || self.binding_passed_as_mut_method_argument(arm.body, binding, registry)
    }

    /// `if let Some(world) = self.world { choice.is_available(world, …) }` needs `&mut self`.
    fn binding_passed_as_mut_method_argument(
        &self,
        expr: &Expression,
        binding: &str,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        let Some(reg) = registry else {
            return false;
        };
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_binding_passed_as_mut_method_argument(s, binding, reg)),
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => arguments.iter().enumerate().any(|(i, (_, a))| {
                matches!(a, Expression::Identifier { name, .. } if name == binding)
                    && self.method_call_argument_expects_mut_borrow(object, method, i, reg)
            }),
            _ => false,
        }
    }

    fn statement_binding_passed_as_mut_method_argument(
        &self,
        stmt: &Statement,
        binding: &str,
        reg: &super::SignatureRegistry,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.binding_passed_as_mut_method_argument(expr, binding, Some(reg))
            }
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                self.binding_passed_as_mut_method_argument(arm.body, binding, Some(reg))
            }),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.binding_passed_as_mut_method_argument(condition, binding, Some(reg))
                    || then_block.iter().any(|s| {
                        self.statement_binding_passed_as_mut_method_argument(s, binding, reg)
                    })
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter().any(|s| {
                            self.statement_binding_passed_as_mut_method_argument(s, binding, reg)
                        })
                    })
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_binding_passed_as_mut_method_argument(s, binding, reg)),
            Statement::For { body, .. } => body
                .iter()
                .any(|s| self.statement_binding_passed_as_mut_method_argument(s, binding, reg)),
            _ => false,
        }
    }

    pub(crate) fn method_call_argument_expects_mut_borrow(
        &self,
        receiver: &Expression,
        method: &str,
        arg_idx: usize,
        reg: &super::SignatureRegistry,
    ) -> bool {
        super::stdlib_method_traits::callable_arg_expects_mut_borrow(
            method,
            self.receiver_type_base_for_method_object(receiver)
                .as_deref(),
            arg_idx,
            false,
            reg,
        )
    }

    fn receiver_type_base_for_method_object(&self, receiver: &Expression) -> Option<String> {
        let ctx = self.self_impl_context.as_ref()?;
        let receiver_ty = self.static_value_type_of_self_rooted_expr(
            ctx.program(),
            &ctx.impl_type_base,
            receiver,
        )?;
        Self::type_base_for_qualified_sig_lookup(&receiver_ty)
    }

    pub(crate) fn expr_calls_mut_self_method_on_identifier(
        &self,
        expr: &Expression,
        id: &str,
        registry: Option<&super::SignatureRegistry>,
        binding_type_bases: Option<&std::collections::HashMap<String, String>>,
    ) -> bool {
        match expr {
            Expression::Block { statements, .. } => {
                self.block_expr_calls_mut_self_on_id(statements.as_slice(), id, registry, binding_type_bases)
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == id {
                        if let Some(reg) = registry {
                            let receiver_base =
                                binding_type_bases.and_then(|m| m.get(id).map(String::as_str));
                            if super::stdlib_method_traits::method_call_mutates_receiver(
                                method,
                                receiver_base,
                                reg,
                            ) {
                                return true;
                            }
                        }
                    }
                }
                self.expr_calls_mut_self_method_on_identifier(
                    object,
                    id,
                    registry,
                    binding_type_bases,
                ) || arguments.iter().any(|(_, a)| {
                    self.expr_calls_mut_self_method_on_identifier(
                        a,
                        id,
                        registry,
                        binding_type_bases,
                    )
                })
            }
            Expression::Binary { left, right, .. } => {
                self.expr_calls_mut_self_method_on_identifier(
                    left,
                    id,
                    registry,
                    binding_type_bases,
                ) || self.expr_calls_mut_self_method_on_identifier(
                    right,
                    id,
                    registry,
                    binding_type_bases,
                )
            }
            Expression::Unary { operand, .. } => self.expr_calls_mut_self_method_on_identifier(
                operand,
                id,
                registry,
                binding_type_bases,
            ),
            Expression::Call { arguments, .. } => arguments.iter().any(|(_, a)| {
                self.expr_calls_mut_self_method_on_identifier(
                    a,
                    id,
                    registry,
                    binding_type_bases,
                )
            }),
            _ => false,
        }
    }

    pub(crate) fn block_expr_calls_mut_self_on_id<'s>(
        &self,
        block: &[&'s Statement<'s>],
        id: &str,
        registry: Option<&super::SignatureRegistry>,
        binding_type_bases: Option<&std::collections::HashMap<String, String>>,
    ) -> bool {
        for s in block {
            match *s {
                Statement::Expression { expr, .. } => {
                    if self.expr_calls_mut_self_method_on_identifier(
                        expr,
                        id,
                        registry,
                        binding_type_bases,
                    ) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expr_calls_mut_self_method_on_identifier(
                        expr,
                        id,
                        registry,
                        binding_type_bases,
                    ) {
                        return true;
                    }
                }
                Statement::Let { value, .. } => {
                    if self.expr_calls_mut_self_method_on_identifier(
                        value,
                        id,
                        registry,
                        binding_type_bases,
                    ) {
                        return true;
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    if self.block_expr_calls_mut_self_on_id(body, id, registry, binding_type_bases)
                    {
                        return true;
                    }
                }
                Statement::For { body, .. } => {
                    if self.block_expr_calls_mut_self_on_id(body, id, registry, binding_type_bases)
                    {
                        return true;
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.expr_calls_mut_self_method_on_identifier(
                        condition,
                        id,
                        registry,
                        binding_type_bases,
                    ) {
                        return true;
                    }
                    if self.block_expr_calls_mut_self_on_id(
                        then_block,
                        id,
                        registry,
                        binding_type_bases,
                    ) {
                        return true;
                    }
                    if let Some(e) = else_block {
                        if self.block_expr_calls_mut_self_on_id(e, id, registry, binding_type_bases)
                        {
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
