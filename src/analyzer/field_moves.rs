//! Field-move facts for parameters (`AnalyzedFunction::field_moved_parameters`): whether a non-Copy field
//! or element of a parameter is moved out by value. One walk, read by the analyzer seed (parameter step 2.5)
//! and by codegen (C5: codegen no longer re-walks the body for it).

use crate::parser::*;
use std::collections::HashSet;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    /// Parameters (other than `self`) with a non-Copy field or element moved out by value.
    pub(super) fn field_moved_parameters(&self, func: &FunctionDecl<'ast>) -> HashSet<String> {
        func.parameters
            .iter()
            .filter(|p| p.name != "self" && self.param_field_is_moved_out(&p.name, &p.type_, &func.body))
            .map(|p| p.name.clone())
            .collect()
    }

    /// The `param.field...` projections that `expr_has_field_move_from_param` treats as moves
    /// (same value positions), so their types can be checked.
    fn collect_value_projections_of_param<'e, 'a>(
        param_name: &str,
        expr: &'e Expression<'a>,
        value_position: bool,
        out: &mut Vec<&'e Expression<'a>>,
    ) {
        let mut each = |e: &'e Expression<'a>, v: bool| Self::collect_value_projections_of_param(param_name, e, v, out);
        match expr {
            Expression::FieldAccess { object, .. } => {
                if value_position && Self::expr_is_field_or_index_rooted_at_param(param_name, expr) {
                    out.push(expr);
                } else if value_position {
                    each(object, true);
                }
            }
            Expression::Index { object, index, .. } => {
                each(object, false);
                each(index, true);
            }
            Expression::Call { arguments, .. } => arguments.iter().for_each(|(_, a)| each(a, true)),
            Expression::MethodCall { object, arguments, .. } => {
                each(object, false);
                arguments.iter().for_each(|(_, a)| each(a, true));
            }
            Expression::StructLiteral { fields, .. } => fields.iter().for_each(|(_, v)| each(v, true)),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                elements.iter().for_each(|e| each(e, true))
            }
            Expression::Binary { left, right, .. } => {
                each(left, false);
                each(right, false);
            }
            Expression::Unary { operand, .. } => each(operand, false),
            Expression::TryOp { expr: operand, .. }
            | Expression::Await { expr: operand, .. }
            | Expression::Cast { expr: operand, .. }
            | Expression::AsyncCall { expr: operand, .. }
            | Expression::SpawnCall { expr: operand, .. }
            | Expression::Scope { body: operand, .. } => each(operand, true),
            Expression::Select { arms, .. } => {
                for a in arms {
                    if let Some(s) = a.source {
                        each(s, false);
                    }
                    each(a.body, true);
                }
            }
            _ => {}
        }
    }


    /// A non-Copy field or element of `param` is moved out by value (the `field_moved_parameters` fact).
    pub(crate) fn param_field_is_moved_out(&self, param_name: &str, param_type: &Type, body: &[&'ast Statement<'ast>]) -> bool {
        self.param_has_field_or_index_move_binding(param_name, body)
            && !self.param_field_move_bindings_are_only_copy_or_text(param_name, param_type, body)
    }

    /// True when every field/index move from `param` is a Copy or Windjammer text field let
    // (`let kind = …`, `let name = tree.nodes[i].name`). Both are readable from `&T`.
    pub(crate) fn param_field_move_bindings_are_only_copy_or_text(
        &self,
        param_name: &str,
        param_type: &Type,
        body: &[&'ast Statement<'ast>],
    ) -> bool {
        let mut saw_benign_let = false;
        for stmt in body {
            if !self.stmt_field_move_bindings_are_only_copy_or_text(
                param_name,
                param_type,
                stmt,
                &mut saw_benign_let,
            ) {
                return false;
            }
        }
        saw_benign_let
    }

    fn stmt_field_move_bindings_are_only_copy_or_text(
        &self,
        param_name: &str,
        param_type: &Type,
        stmt: &Statement<'ast>,
        saw_benign_let: &mut bool,
    ) -> bool {
        match stmt {
            Statement::Let { value, .. } => {
                if !Self::expr_is_field_move_from_param(param_name, value) {
                    return true;
                }
                if self
                    .infer_projected_field_type(param_name, param_type, value)
                    .is_some_and(|ty| {
                        Self::is_windjammer_text_param_type(&ty) || self.is_copy_type(&ty)
                    })
                {
                    *saw_benign_let = true;
                    true
                } else {
                    false
                }
            }
            Statement::Return {
                value: Some(expr), ..
            }
            | Statement::Expression { expr, .. } => {
                // Non-let field moves still consume, unless every projected value is Copy
                // (`same(db.last, 3)` copies `last` out of `&Store`; nothing is moved).
                if !Self::expr_is_field_move_from_param(param_name, expr) {
                    return true;
                }
                let mut projections = Vec::new();
                Self::collect_value_projections_of_param(param_name, expr, true, &mut projections);
                let all_copy = !projections.is_empty()
                    && projections.iter().all(|p| {
                        self.infer_projected_field_type(param_name, param_type, p)
                            .is_some_and(|ty| self.is_copy_type(&ty))
                    });
                if all_copy {
                    *saw_benign_let = true;
                }
                all_copy
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().all(|s| {
                    self.stmt_field_move_bindings_are_only_copy_or_text(
                        param_name,
                        param_type,
                        s,
                        saw_benign_let,
                    )
                }) && else_block.as_ref().is_none_or(|b| {
                    b.iter().all(|s| {
                        self.stmt_field_move_bindings_are_only_copy_or_text(
                            param_name,
                            param_type,
                            s,
                            saw_benign_let,
                        )
                    })
                })
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::Loop { body, .. } => body.iter().all(|s| {
                self.stmt_field_move_bindings_are_only_copy_or_text(
                    param_name,
                    param_type,
                    s,
                    saw_benign_let,
                )
            }),
            Statement::Match { arms, .. } => arms.iter().all(|arm| {
                if !Self::expr_is_field_move_from_param(param_name, &arm.body) {
                    return true;
                }
                if self
                    .infer_projected_field_type(param_name, param_type, &arm.body)
                    .is_some_and(|ty| {
                        Self::is_windjammer_text_param_type(&ty) || self.is_copy_type(&ty)
                    })
                {
                    *saw_benign_let = true;
                    true
                } else {
                    false
                }
            }),
            _ => true,
        }
    }
}
