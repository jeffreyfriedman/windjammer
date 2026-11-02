//! Struct-literal field coercions that look inside `if` expressions.

use crate::parser::{Expression, Statement, Type};

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Names of the identifiers that end the arms of `if c { a } else { b }` (both arms), if every arm is one.
    fn if_arm_identifier_tails(expr: &Expression<'ast>) -> Option<Vec<String>> {
        let Expression::Block { statements, .. } = expr else { return None };
        let [Statement::If { then_block, else_block: Some(else_block), .. }] = statements.as_slice() else {
            return None;
        };
        let tail = |b: &[&Statement<'ast>]| match b.last() {
            Some(Statement::Expression { expr: Expression::Identifier { name, .. }, .. }) => Some(name.clone()),
            _ => None,
        };
        Some(vec![tail(then_block)?, tail(else_block)?])
    }

    /// `message: if ok { a_param } else { b_param }` with demoted `&str` params: each arm's tail is
    /// converted where the field needs an owned `String`.
    pub(in crate::codegen::rust) fn coerce_if_arm_text_tails(
        &self,
        expr: &Expression<'ast>,
        field_name: &str,
        mut expr_str: String,
    ) -> String {
        let Some(arm_names) = Self::if_arm_identifier_tails(expr) else { return expr_str };
        let field_is_string = self
            .current_struct_literal_name
            .as_deref()
            .and_then(|sn| self.lookup_struct_field_types(sn))
            .and_then(|fields| fields.get(field_name).cloned())
            .is_some_and(|ft| matches!(ft, Type::String) || matches!(ft, Type::Custom(ref n) if n == "string" || n == "String"));
        if !field_is_string {
            return expr_str;
        }
        for id in arm_names {
            let demoted_text = (self.emitted_rust_ref_formals.contains(&id) || self.str_ref_optimized_params.contains(&id))
                && self
                    .current_function_params
                    .iter()
                    .any(|p| p.name == id && super::types::is_windjammer_text_type(&p.type_));
            if demoted_text {
                expr_str = expr_str
                    .lines()
                    .map(|l| {
                        if l.trim() == id {
                            format!("{}{}.to_string()", &l[..l.len() - l.trim_start().len()], id)
                        } else {
                            l.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }
        expr_str
    }
}
