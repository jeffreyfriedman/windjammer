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

    /// C3: the receiver's resolved type defines `method` in its own impl (`Pool::contains`, `Table::find`), so the
    /// call is that method, never the std one of the same name (`str::contains` Pattern), whatever the binding is.
    pub(in crate::codegen::rust) fn type_defines_own_method(&self, receiver_type_name: Option<&str>, method: &str) -> bool {
        receiver_type_name.is_some_and(|rt| {
            let base = rt.split('<').next().unwrap_or(rt);
            self.struct_method_ast_formal_param_types.get(base).is_some_and(|m| m.contains_key(method))
        })
    }

    /// The type a receiver is built from by declaration when inference has no type for it: `RailItem::new(..)` on a
    /// type this file declares or imports (`use dep::RailItem`). Its methods are `RailItem`'s, wherever its crate's
    /// signatures are; a same-named method of a type of another crate is not one of them.
    pub(in crate::codegen::rust) fn receiver_static_call_type(&self, object: &Expression<'ast>) -> Option<String> {
        let Expression::Call { function, .. } = object else { return None };
        let name = super::ast_utilities::extract_function_name(function);
        let (ty, _) = name.rsplit_once("::")?;
        let leaf = ty.rsplit("::").next()?;
        self.idiom.file_scope_type_names.contains(leaf).then(|| leaf.to_string())
    }
}
