//! Which function a method call on an identifier names: the module's function or the value's method.

use crate::parser::Expression;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Registry key for `object.method(..)`. A local binding named like a std module (`let mut fs = Disk::new()`) is
    /// a value: its key follows the receiver's type. Otherwise a module-looking identifier names the module's function.
    pub(in crate::codegen::rust) fn qualified_method_key(
        &self,
        receiver_type_name: Option<&str>,
        object: &Expression<'ast>,
        method: &str,
    ) -> String {
        if let Expression::Identifier { name, .. } = object {
            if self.identifier_is_local_binding(name) {
                return match receiver_type_name {
                    Some(tn) if tn.chars().next().is_some_and(|c| c.is_ascii_uppercase()) => format!("{tn}::{method}"),
                    _ => method.to_string(),
                };
            }
        }
        super::stdlib_method_traits::module_qualified_method_name(receiver_type_name, object, method, |name| {
            self.is_imported_runtime_std_module(name)
        })
    }
}

