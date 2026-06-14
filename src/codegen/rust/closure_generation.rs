//! Closure expression generation
//!
//! Handles generation of:
//! - Closure expressions (|params| body)
//! - Smart `move` inference based on capture analysis
//!
//! THE WINDJAMMER WAY: The compiler infers `move` automatically.
//! - Closures that capture outer variables → `move` (ownership transfer)
//! - Closures that only use their own params → no `move` (pure functions)
//! - Closures that capture `self` → no `move` (borrow from method receiver)
//! - Compiler-generated wrappers (__params) → always `move`

use crate::parser::{Expression, Statement};

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for closure expression |params| body
    pub(in crate::codegen::rust) fn generate_closure(
        &mut self,
        parameters: &[String],
        body: &Expression<'ast>,
    ) -> String {
        let params = parameters.join(", ");

        let is_compiler_generated = parameters.iter().any(|p| p.starts_with("__"));
        let captures_self = self.expression_references_self(body);
        let captures_outer = self.closure_captures_outer_variables(parameters, body);

        // For user-written closures, set flag and track params to suppress transformations
        let prev_in_user_closure = self.in_user_written_closure;
        let mut prev_closure_params = None;
        if !is_compiler_generated {
            self.in_user_written_closure = true;
            prev_closure_params = Some(std::mem::take(&mut self.user_closure_params));
            for param in parameters {
                self.user_closure_params.insert(param.clone());
            }
        }

        let body_str = self.generate_expression(body);

        if !is_compiler_generated {
            self.in_user_written_closure = prev_in_user_closure;
            if let Some(prev_params) = prev_closure_params {
                self.user_closure_params = prev_params;
            }
        }

        let needs_move = if captures_self {
            false
        } else if is_compiler_generated {
            true
        } else {
            captures_outer
        };

        if needs_move {
            format!("move |{}| {}", params, body_str)
        } else {
            format!("|{}| {}", params, body_str)
        }
    }

    /// Determine if a closure body references identifiers that aren't its own parameters,
    /// meaning it captures variables from the enclosing scope.
    fn closure_captures_outer_variables(
        &self,
        parameters: &[String],
        body: &Expression<'ast>,
    ) -> bool {
        let param_set: std::collections::HashSet<&str> =
            parameters.iter().map(|s| s.as_str()).collect();
        self.expr_has_free_identifier(&param_set, body)
    }

    fn expr_has_free_identifier(
        &self,
        bound: &std::collections::HashSet<&str>,
        expr: &Expression<'ast>,
    ) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name != "self" && !bound.contains(name.as_str()),
            Expression::FieldAccess { object, .. } => self.expr_has_free_identifier(bound, object),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expr_has_free_identifier(bound, object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_has_free_identifier(bound, arg))
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let fn_captures = match &**function {
                    Expression::Identifier { .. } => false,
                    _ => self.expr_has_free_identifier(bound, function),
                };
                fn_captures
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expr_has_free_identifier(bound, arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expr_has_free_identifier(bound, left)
                    || self.expr_has_free_identifier(bound, right)
            }
            Expression::Unary { operand, .. } => self.expr_has_free_identifier(bound, operand),
            Expression::Index { object, index, .. } => {
                self.expr_has_free_identifier(bound, object)
                    || self.expr_has_free_identifier(bound, index)
            }
            Expression::Block { statements, .. } => {
                self.ref_stmts_have_free_identifier(bound, statements)
            }
            Expression::Closure {
                parameters: inner_params,
                body: inner_body,
                ..
            } => {
                let mut extended = bound.clone();
                for p in inner_params {
                    extended.insert(p.as_str());
                }
                self.expr_has_free_identifier(&extended, inner_body)
            }
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, val)| self.expr_has_free_identifier(bound, val)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|el| self.expr_has_free_identifier(bound, el)),
            Expression::Cast { expr: inner, .. } => self.expr_has_free_identifier(bound, inner),
            Expression::Range { start, end, .. } => {
                self.expr_has_free_identifier(bound, start)
                    || self.expr_has_free_identifier(bound, end)
            }
            _ => false,
        }
    }

    fn ref_stmts_have_free_identifier<'b>(
        &self,
        bound: &std::collections::HashSet<&'b str>,
        stmts: &'b [&'b Statement<'ast>],
    ) -> bool {
        let mut local_bound = bound.clone();
        for stmt in stmts {
            if self.stmt_has_free_identifier(&mut local_bound, stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_has_free_identifier<'b>(
        &self,
        local_bound: &mut std::collections::HashSet<&'b str>,
        stmt: &'b Statement<'ast>,
    ) -> bool {
        match stmt {
            Statement::Let { pattern, value, .. } => {
                if self.expr_has_free_identifier(local_bound, value) {
                    return true;
                }
                Self::bind_pattern(local_bound, pattern);
                false
            }
            Statement::Assignment { value, .. } => {
                self.expr_has_free_identifier(local_bound, value)
            }
            Statement::Return { value, .. } => {
                value.is_some_and(|v| self.expr_has_free_identifier(local_bound, v))
            }
            Statement::Expression { expr, .. } => self.expr_has_free_identifier(local_bound, expr),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                if self.expr_has_free_identifier(local_bound, condition) {
                    return true;
                }
                if self.ref_stmts_have_free_identifier(local_bound, then_block) {
                    return true;
                }
                if let Some(block) = else_block {
                    if self.ref_stmts_have_free_identifier(local_bound, block) {
                        return true;
                    }
                }
                false
            }
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                if self.expr_has_free_identifier(local_bound, iterable) {
                    return true;
                }
                let mut for_bound = local_bound.clone();
                Self::bind_pattern(&mut for_bound, pattern);
                self.ref_stmts_have_free_identifier(&for_bound, body)
            }
            Statement::Match { value, arms, .. } => {
                if self.expr_has_free_identifier(local_bound, value) {
                    return true;
                }
                arms.iter()
                    .any(|arm| self.expr_has_free_identifier(local_bound, arm.body))
            }
            _ => false,
        }
    }

    fn bind_pattern<'b>(
        bound: &mut std::collections::HashSet<&'b str>,
        pattern: &'b crate::parser::Pattern,
    ) {
        use crate::parser::Pattern;
        match pattern {
            Pattern::Identifier(name) => {
                bound.insert(name.as_str());
            }
            Pattern::MutBinding(name) => {
                bound.insert(name.as_str());
            }
            Pattern::Ref(name) => {
                bound.insert(name.as_str());
            }
            Pattern::RefMut(name) => {
                bound.insert(name.as_str());
            }
            Pattern::Tuple(pats) => {
                for p in pats {
                    Self::bind_pattern(bound, p);
                }
            }
            Pattern::Or(pats) => {
                for p in pats {
                    Self::bind_pattern(bound, p);
                }
            }
            Pattern::Reference(inner) => {
                Self::bind_pattern(bound, inner);
            }
            Pattern::EnumVariant(_, _) => {}
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
    }
}
