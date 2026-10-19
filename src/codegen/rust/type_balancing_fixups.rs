//! Equality-operand balancing, plus literal-comparison fixups applied after it.

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn balance_eq_operands_checked(
        &self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
        left_str: &mut String,
        right_str: &mut String,
        is_ordering_compare: bool,
    ) {
        self.balance_eq_operands_for_rust(left, right, left_str, right_str, is_ordering_compare);
        // `tok == "lit"`: a compiler-added `*` on a plain identifier would compare `str` with `&str`
        // when the binding is a `&str` (`for tok in text.split(..)`). `String == &str`,
        // `&String == &str` and `&str == &str` all type-check without it.
        let is_str_literal = |e: &Expression<'ast>| {
            matches!(e, Expression::Literal { value: crate::parser::Literal::String(_), .. })
        };
        if is_str_literal(right) && matches!(left, Expression::Identifier { .. }) && left_str.starts_with('*') {
            *left_str = left_str[1..].to_string();
        }
        if is_str_literal(left) && matches!(right, Expression::Identifier { .. }) && right_str.starts_with('*') {
            *right_str = right_str[1..].to_string();
        }
    }
}
