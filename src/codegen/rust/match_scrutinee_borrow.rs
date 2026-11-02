//! When matching an `Option` field of an owned parameter must borrow the field.

use crate::parser::{Expression, Type};

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// `match cmd.replace { Some(r) => .. }` where `cmd` is an owned parameter and the match runs inside a loop:
    /// matching by value would move the payload out on the first iteration (E0382 "moved in previous iteration").
    /// Borrow the field (`&cmd.replace`) when its payload is not `Copy`.
    pub(in crate::codegen::rust) fn owned_param_option_in_loop_prefix(&self, value: &Expression<'ast>) -> &'static str {
        if self.loop_body_depth == 0 {
            return "";
        }
        let Some(root) = self.root_identifier_of_field_or_index_chain(value) else { return "" };
        let owned_param = self.current_function_params.iter().any(|p| p.name == root)
            && !self.inferred_borrowed_params.contains(root)
            && !self.inferred_mut_borrowed_params.contains(root);
        match self.infer_expression_type(value) {
            Some(Type::Option(inner)) if owned_param && !self.is_type_copy(&inner) => "&",
            _ => "",
        }
    }
}
