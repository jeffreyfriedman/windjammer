//! Initialiser of a `let` without a type annotation.

use crate::parser::Expression;

use super::{string_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate the initialiser of an unannotated `let`. When the binding is ascribed `: String` because the
    /// initialiser is a `match` / `if` yielding string literals, every literal arm is converted to an owned
    /// `String` (otherwise the ascription is violated by `&str` arms: E0308).
    pub(in crate::codegen::rust) fn generate_untyped_let_value(&mut self, value: &Expression<'ast>) -> String {
        let owned_arms = matches!(value, Expression::Block { .. })
            && string_utilities::untyped_let_rhs_needs_string_ascription(value);
        let prev = self.coerce_string_literals_to_owned;
        if owned_arms {
            self.coerce_string_literals_to_owned = true;
        }
        let s = self.generate_expression(value);
        self.coerce_string_literals_to_owned = prev;
        // `let f = h.run` of a field that holds a boxed callable (stored_callables): borrow it, a box is not Copy
        if let Expression::FieldAccess { object, field, .. } = value {
            if self.declared_field_type(object, field).is_some_and(|t| crate::codegen::rust::stored_callables::holds_boxed_callable(&t))
                && self.place_is_borrowed(object)
            {
                return format!("&{s}");
            }
        }
        // CLONE-USE-AFTER-MOVE: `let t = s` with `s` read again copies `s`.
        self.copy_if_read_again(value, s)
    }
}
