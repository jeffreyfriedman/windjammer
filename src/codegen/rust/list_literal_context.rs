//! ADR-0014: a list literal `[a, b]` is a growable list (`Vec`) unless the place is a fixed array.

use crate::parser::{Expression, Statement, Type};

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// `if c { [a, b] } else { [c, d] }` (every arm's tail is a list literal): the arms are lists too.
    fn if_arms_yield_list_literals(value: &Expression<'ast>) -> bool {
        fn tail_is_list(block: &[&Statement<'_>]) -> bool {
            matches!(
                block.last(),
                Some(Statement::Expression { expr: Expression::Array { .. }, .. })
            )
        }
        let Expression::Block { statements, .. } = value else { return false };
        let [Statement::If { then_block, else_block: Some(else_block), .. }] = statements.as_slice() else {
            return false;
        };
        tail_is_list(then_block) && tail_is_list(else_block)
    }

    /// Addresses (element slices) of the list literals in return position of `body`: the tail
    /// expression, `return` values, and the tails of `if`/`match` arms and blocks in those positions.
    /// Closures have their own return position and are not entered.
    pub(in crate::codegen::rust) fn collect_return_position_lists(
        body: &[&Statement<'_>],
    ) -> std::collections::HashSet<usize> {
        fn stmts(list: &[&Statement<'_>], tail: bool, out: &mut std::collections::HashSet<usize>) {
            for (i, st) in list.iter().enumerate() {
                let last = tail && i + 1 == list.len();
                match st {
                    Statement::Return { value: Some(e), .. } => expr(e, true, out),
                    Statement::Expression { expr: e, .. } => expr(e, last, out),
                    Statement::If { then_block, else_block, .. } => {
                        stmts(then_block, last, out);
                        if let Some(b) = else_block {
                            stmts(b, last, out);
                        }
                    }
                    Statement::Match { arms, .. } => {
                        for a in arms {
                            expr(a.body, last, out);
                        }
                    }
                    Statement::For { body, .. }
                    | Statement::While { body, .. }
                    | Statement::Loop { body, .. } => stmts(body, false, out),
                    _ => {}
                }
            }
        }
        fn expr(e: &Expression<'_>, tail: bool, out: &mut std::collections::HashSet<usize>) {
            match e {
                Expression::Array { elements, .. } if tail && !elements.is_empty() => {
                    out.insert(elements.as_ptr() as usize);
                }
                Expression::Block { statements, .. } => stmts(statements, tail, out),
                _ => {}
            }
        }
        let mut out = std::collections::HashSet::new();
        stmts(body, true, &mut out);
        out
    }

    /// Should list literals in `value` be generated as `Vec`s? `declared` is the annotated type, if any.
    pub(in crate::codegen::rust) fn list_literal_wanted(value: &Expression<'ast>, declared: Option<&Type>) -> bool {
        (matches!(value, Expression::Array { .. }) || Self::if_arms_yield_list_literals(value))
            && declared.is_none_or(crate::type_classification::type_is_vec_container)
    }

    /// Generate `value` assigned to a place of type `target`: list literals are `Vec`s unless the place is a fixed array.
    pub(in crate::codegen::rust) fn generate_with_list_context(&mut self, value: &Expression<'ast>, target: Option<&Type>) -> String {
        let prev = self.array_literal_as_list;
        self.array_literal_as_list = matches!(value, Expression::Array { .. }) && !matches!(target, Some(Type::Array(..)));
        let s = self.generate_expression(value);
        self.array_literal_as_list = prev;
        s
    }
}
