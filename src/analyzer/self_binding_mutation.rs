//! Loop bindings and tracing expressions rooted in `self`.
use crate::parser::*;

use super::Analyzer;
impl<'ast> Analyzer<'ast> {
    fn for_loop_body_mutates_element_of_self_iterable(
        &self,
        pattern: &Pattern,
        iterable: &Expression<'ast>,
        body: &[&Statement<'ast>],
    ) -> bool {
        // TDD FIX: Handle tuple patterns (id, val) not just simple identifiers
        // Extract ALL bindings from the pattern and check if ANY are mutated
        let loop_vars = match pattern {
            Pattern::Identifier(var) => vec![var.clone()],
            Pattern::Tuple(patterns) => {
                let mut vars = Vec::new();
                for pat in patterns {
                    if let Pattern::Identifier(var) = pat {
                        vars.push(var.clone());
                    }
                }
                vars
            }
            _ => return false,
        };

        let peeled = Self::peel_ref_expr(iterable);
        if !self.expression_traces_to_self(peeled) {
            return false;
        }

        // Check if ANY binding is mutated
        loop_vars.iter().any(|loop_var| {
            body.iter()
                .any(|s| self.statement_tree_mutates_binding(s, loop_var.as_str()))
        })
    }

    fn assignment_target_starts_with_var(expr: &Expression, var: &str) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == var,
            Expression::FieldAccess { object, .. } => {
                Self::assignment_target_starts_with_var(object, var)
            }
            Expression::Index { object, .. } => {
                Self::assignment_target_starts_with_var(object, var)
            }
            // TDD FIX: Handle dereference assignments (*val = ..., *var.field = ...)
            // For: *val = value, need to detect that 'val' is the target variable
            Expression::Unary {
                op: UnaryOp::Deref,
                operand,
                ..
            } => Self::assignment_target_starts_with_var(operand, var),
            _ => false,
        }
    }

    /// Whether `stmt` mutates `var` or a field/index chain rooted at `var` (loop element patterns).
    fn statement_tree_mutates_binding(&self, stmt: &Statement, var: &str) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                Self::assignment_target_starts_with_var(target, var)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.statement_tree_mutates_binding(s, var))
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_tree_mutates_binding(s, var))
                    })
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.statement_tree_mutates_binding(s, var)),
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                self.for_loop_body_mutates_element_of_self_iterable(pattern, iterable, body)
                    || body
                        .iter()
                        .any(|s| self.statement_tree_mutates_binding(s, var))
            }
            Statement::Match { arms, .. } => arms
                .iter()
                .any(|arm| self.expression_tree_mutates_binding(arm.body, var)),
            Statement::Expression { expr, .. } => self.expression_tree_mutates_binding(expr, var),
            Statement::Let {
                value, else_block, ..
            } => {
                self.expression_tree_mutates_binding(value, var)
                    || else_block.as_ref().is_some_and(|block| {
                        block
                            .iter()
                            .any(|s| self.statement_tree_mutates_binding(s, var))
                    })
            }
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_tree_mutates_binding(expr, var),
            _ => false,
        }
    }

    fn expression_tree_mutates_binding(&self, expr: &Expression, var: &str) -> bool {
        match expr {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.statement_tree_mutates_binding(s, var)),
            _ => false,
        }
    }

    /// Check if an expression traces back to `self` through a chain of field accesses.
    /// Returns true for: self.field, self.field.subfield, self.field[i], etc.
    fn expression_traces_to_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    name == "self"
                } else {
                    self.expression_traces_to_self(object)
                }
            }
            // TDD FIX: self.factions[i].method() - Index on self.field traces to self
            Expression::Index { object, .. } => self.expression_traces_to_self(object),
            _ => false,
        }
    }
}
