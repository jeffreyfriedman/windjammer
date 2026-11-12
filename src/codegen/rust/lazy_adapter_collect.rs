//! `let names: Vec<string> = pairs.map(|p| p.1)`: in Windjammer an adapter called on a list yields a list, in Rust it
//! is a lazy iterator, so a `Vec` annotation on the binding materialises it.

use crate::parser::{Expression, Type};
use crate::ir::stdlib_tables::{surface_type_name, CoreSurface, ITERATOR};

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Append `.collect::<Vec<_>>()` to `value_str` when `value` is a call of an `Iterator` adapter (declared in
    /// `std/rust_core.wji` as returning an `Iterator`) and the binding is declared as a list.
    pub(in crate::codegen::rust) fn collect_declared_list_from_adapter(&self, value: &Expression<'ast>, declared: &Type, value_str: &mut String) {
        let Expression::MethodCall { method, .. } = value else { return };
        if !crate::type_classification::type_is_vec_container(declared) || value_str.ends_with(">()") {
            return;
        }
        let yields_iterator = self.core_surface()
            .declared(ITERATOR, method)
            .and_then(|(_, _, _, ret)| ret)
            .is_some_and(|ret| surface_type_name(&ret).is_some_and(|head| head.eq(ITERATOR)));
        if yields_iterator {
            value_str.push_str(".collect::<Vec<_>>()");
        }
    }
}
