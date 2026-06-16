use crate::codegen::rust::self_analysis;
use crate::parser::*;

use super::CodeGenerator;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// TDD FIX for E0507: Check if for-loop should borrow the iterable
    /// Only borrow if the base object is borrowed (not owned)
    pub(crate) fn should_borrow_for_iteration(&self, iterable: &Expression) -> bool {
        match iterable {
            Expression::Unary {
                op: UnaryOp::Ref | UnaryOp::MutRef,
                ..
            } => false,
            // Subscript/field iteration must borrow to avoid partial moves (E0507).
            // Example: `for dep in pass_defs[i].dependencies` inside an outer loop.
            Expression::Index { .. } => true,
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        if self.inferred_mut_borrowed_params.contains("self") {
                            return false;
                        }
                        return self.inferred_borrowed_params.contains("self");
                    }
                    return self.inferred_borrowed_params.contains(name)
                        || self.inferred_mut_borrowed_params.contains(name)
                        || self.borrowed_iterator_vars.contains(name);
                }
                if let Expression::FieldAccess { .. } = &**object {
                    return self.should_borrow_for_iteration(object);
                }
                if matches!(&**object, Expression::Index { .. }) {
                    return true;
                }
                false
            }
            Expression::Identifier { name, .. } => self.for_loop_borrow_needed.contains(name),
            _ => false,
        }
    }

    /// Check if we're iterating over a borrowed collection
    pub(crate) fn is_iterating_over_borrowed(&self, iterable: &Expression) -> bool {
        match iterable {
            Expression::Unary { op, .. } => {
                matches!(op, UnaryOp::Ref | UnaryOp::MutRef)
            }
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        return self.inferred_borrowed_params.contains("self")
                            || self.inferred_mut_borrowed_params.contains("self");
                    }
                    return self.inferred_borrowed_params.contains(name)
                        || self.inferred_mut_borrowed_params.contains(name)
                        || self.borrowed_iterator_vars.contains(name);
                }
                if let Expression::FieldAccess { .. } = &**object {
                    return self.should_borrow_for_iteration(object);
                }
                false
            }
            Expression::Identifier { name, .. } => {
                self.current_function_params.iter().any(|p| {
                    &p.name == name
                        && (matches!(p.ownership, crate::parser::OwnershipHint::Ref)
                            || matches!(
                                &p.type_,
                                crate::parser::Type::Reference(_)
                                    | crate::parser::Type::MutableReference(_)
                            ))
                }) || self.inferred_borrowed_params.contains(name)
                    || self.borrowed_iterator_vars.contains(name)
                    || self.local_var_types.get(name).is_some_and(|t| {
                        matches!(
                            t,
                            crate::parser::Type::Reference(_)
                                | crate::parser::Type::MutableReference(_)
                        )
                    })
            }
            Expression::MethodCall { method, .. } => {
                crate::codegen::rust::stdlib_method_traits::method_returns_iterator(method)
            }
            _ => false,
        }
    }

    /// Check if a loop body modifies a variable
    pub(crate) fn loop_body_modifies_variable(
        &self,
        body: &[&'ast Statement<'ast>],
        var_name: &str,
    ) -> bool {
        for stmt in body {
            if self.statement_modifies_variable(stmt, var_name) {
                return true;
            }
        }
        false
    }

    /// When iterating `self.field` with owned self (not in borrowed params),
    /// check if the loop body uses `self` in any way (method calls, field access).
    /// If so, we must borrow the iterable (`&self.field`) to avoid partial move (E0382).
    pub(crate) fn self_field_iterable_needs_borrow(
        &self,
        iterable: &Expression,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        let is_self_field = matches!(
            iterable,
            Expression::FieldAccess { object, .. }
                if matches!(&**object, Expression::Identifier { name, .. } if name == "self")
        );
        if !is_self_field {
            return false;
        }
        if self.inferred_borrowed_params.contains("self")
            || self.inferred_mut_borrowed_params.contains("self")
        {
            return false;
        }
        Self::variable_used_in_statements(body, "self")
    }

    /// For `for x in coll` when `coll` is borrowed (`&self.field`): if the body calls a method on
    /// `x` whose inferred receiver is `&mut self` (e.g. `System::update` on `Box<dyn System>`),
    /// the iterable must be `&mut coll`, not `&coll`.
    pub(crate) fn loop_body_calls_mut_dispatch_method(
        &self,
        iterable: &Expression<'ast>,
        body: &[&'ast Statement<'ast>],
        loop_var: &str,
    ) -> bool {
        let Some(iter_t) = self.infer_expression_type(iterable) else {
            return false;
        };
        let Some(elem) = Self::extract_iterator_element_type(&iter_t) else {
            return false;
        };
        body.iter()
            .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, &elem))
    }

    fn peel_dispatch_element_type(ty: &Type) -> &Type {
        match ty {
            Type::Parameterized(name, args) if Self::type_base_is_box(name) && args.len() == 1 => {
                Self::peel_dispatch_element_type(&args[0])
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::peel_dispatch_element_type(inner.as_ref())
            }
            _ => ty,
        }
    }

    fn type_base_is_box(name: &str) -> bool {
        name == "Box" || name.ends_with("::Box")
    }

    fn trait_method_self_ownership_for_loop(
        &self,
        trait_name: &str,
        method: &str,
    ) -> Option<crate::analyzer::OwnershipMode> {
        let from_map =
            |m: &std::collections::HashMap<String, crate::analyzer::AnalyzedFunction<'ast>>| {
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

    pub(crate) fn method_requires_mut_receiver_for_element_type(
        &self,
        elem: &Type,
        method: &str,
    ) -> bool {
        let peeled = Self::peel_dispatch_element_type(elem);
        match peeled {
            Type::TraitObject(trait_name) => self
                .trait_method_self_ownership_for_loop(trait_name, method)
                .is_some_and(|o| o == crate::analyzer::OwnershipMode::MutBorrowed),
            Type::Custom(type_name) => self
                .signature_registry
                .get_signature(&format!("{}::{}", type_name, method))
                .filter(|s| s.has_self_receiver)
                .and_then(|s| s.param_ownership.first().copied())
                .is_some_and(|o| o == crate::analyzer::OwnershipMode::MutBorrowed),
            _ => false,
        }
    }

    fn stmt_contains_mut_dispatch_on_var(
        &self,
        stmt: &Statement<'ast>,
        loop_var: &str,
        elem: &Type,
    ) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expr_contains_mut_dispatch_on_var(expr, loop_var, elem)
            }
            Statement::Let {
                value, else_block, ..
            } => {
                self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
                    })
            }
            Statement::Const { value, .. } | Statement::Static { value, .. } => {
                self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
            }
            Statement::Assignment { target, value, .. } => {
                self.expr_contains_mut_dispatch_on_var(target, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_contains_mut_dispatch_on_var(expr, loop_var, elem),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_contains_mut_dispatch_on_var(condition, loop_var, elem)
                    || then_block
                        .iter()
                        .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
                    })
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr_contains_mut_dispatch_on_var(condition, loop_var, elem)
                    || body
                        .iter()
                        .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
            }
            Statement::For { iterable, body, .. } => {
                self.expr_contains_mut_dispatch_on_var(iterable, loop_var, elem)
                    || body
                        .iter()
                        .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem))
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => body
                .iter()
                .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem)),
            Statement::Match { value, arms, .. } => {
                self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
                    || arms.iter().any(|a| {
                        a.guard.is_some_and(|g| {
                            self.expr_contains_mut_dispatch_on_var(g, loop_var, elem)
                        }) || self.expr_contains_mut_dispatch_on_var(a.body, loop_var, elem)
                    })
            }
            Statement::Defer { statement, .. } => {
                self.stmt_contains_mut_dispatch_on_var(statement, loop_var, elem)
            }
            _ => false,
        }
    }

    fn expr_contains_mut_dispatch_on_var(
        &self,
        expr: &Expression<'ast>,
        loop_var: &str,
        elem: &Type,
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
                        && self.method_requires_mut_receiver_for_element_type(elem, method)
                    {
                        return true;
                    }
                }
                if self.expr_contains_mut_dispatch_on_var(object, loop_var, elem) {
                    return true;
                }
                arguments
                    .iter()
                    .any(|(_, a)| self.expr_contains_mut_dispatch_on_var(a, loop_var, elem))
            }
            Expression::Binary { left, right, .. } => {
                self.expr_contains_mut_dispatch_on_var(left, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(right, loop_var, elem)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.expr_contains_mut_dispatch_on_var(function, loop_var, elem)
                    || arguments
                        .iter()
                        .any(|(_, a)| self.expr_contains_mut_dispatch_on_var(a, loop_var, elem))
            }
            Expression::Unary { operand, .. } => {
                self.expr_contains_mut_dispatch_on_var(operand, loop_var, elem)
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.stmt_contains_mut_dispatch_on_var(s, loop_var, elem)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                self.expr_contains_mut_dispatch_on_var(k, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(v, loop_var, elem)
            }),
            Expression::FieldAccess { object, .. } => {
                self.expr_contains_mut_dispatch_on_var(object, loop_var, elem)
            }
            Expression::Index { object, index, .. } => {
                self.expr_contains_mut_dispatch_on_var(object, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(index, loop_var, elem)
            }
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expr_contains_mut_dispatch_on_var(v, loop_var, elem)),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| self.expr_contains_mut_dispatch_on_var(e, loop_var, elem)),
            Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
                self.expr_contains_mut_dispatch_on_var(expr, loop_var, elem)
            }
            Expression::Closure { body, .. } => {
                self.expr_contains_mut_dispatch_on_var(body, loop_var, elem)
            }
            Expression::Cast { expr, .. } => {
                self.expr_contains_mut_dispatch_on_var(expr, loop_var, elem)
            }
            Expression::Range { start, end, .. } => {
                self.expr_contains_mut_dispatch_on_var(start, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(end, loop_var, elem)
            }
            Expression::MacroInvocation { args, .. } => args
                .iter()
                .any(|a| self.expr_contains_mut_dispatch_on_var(a, loop_var, elem)),
            Expression::ChannelSend { channel, value, .. } => {
                self.expr_contains_mut_dispatch_on_var(channel, loop_var, elem)
                    || self.expr_contains_mut_dispatch_on_var(value, loop_var, elem)
            }
            Expression::ChannelRecv { channel, .. } => {
                self.expr_contains_mut_dispatch_on_var(channel, loop_var, elem)
            }
            _ => false,
        }
    }

    fn statement_modifies_variable(&self, stmt: &Statement, var_name: &str) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                self_analysis::expression_references_variable_or_field(target, var_name)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_modifies_variable(s, var_name))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_modifies_variable(s, var_name))
                    })
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_modifies_variable(s, var_name)),
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                if let Expression::Block { statements, .. } = arm.body {
                    statements
                        .iter()
                        .any(|s| self.statement_modifies_variable(s, var_name))
                } else {
                    false
                }
            }),
            _ => false,
        }
    }
}
