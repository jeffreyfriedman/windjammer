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
