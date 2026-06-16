//! Method call expression generation
//!
//! Split across `receiver`, `signature_resolution`, `arguments`, and `finalize`.

use crate::parser::*;

mod arguments;
mod finalize;
mod receiver;
mod signature_resolution;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a method call expression
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn generate_method_call_expression(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
        type_args: &Option<Vec<Type>>,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> String {
        if super::rust_stdlib_annotations::is_strip_redundant(method) && arguments.is_empty() {
            if let Expression::Identifier { name, .. } = object {
                let is_borrowed = self.inferred_borrowed_params.contains(name.as_str());
                if is_borrowed {
                    return self.generate_expression(object);
                }
            }
        }

        if arguments.is_empty() {
            let receiver_type = self
                .infer_type_name(object)
                .or_else(|| self.infer_indexed_element_type_name(object));
            if let Some(receiver_type) = receiver_type {
                let recv_is_ref = matches!(
                    self.infer_expression_type(object).as_ref(),
                    Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                ) || matches!(
                    object,
                    Expression::Identifier { name, .. }
                        if self.inferred_borrowed_params.contains(name)
                            || self.inferred_mut_borrowed_params.contains(name)
                );
                if let Some(fields) = self.lookup_struct_field_types(&receiver_type) {
                    if fields.get(method).is_some_and(|ty| self.is_type_copy(ty)) {
                        let has_method = self.method_exists_on_type_name(&receiver_type, method);
                        // GPU buffer types expose `buffer_id()` methods that take owned `self`
                        // but call sites hold `&StorageRead<T>` — use the Copy field instead.
                        if recv_is_ref || !has_method {
                            return format!("{}.{}", self.generate_expression(object), method);
                        }
                    }
                }
            }
        }

        // TDD FIX: Upgrade HashMap.get() to get_mut() when the bound value is mutated downstream
        let effective_method = if method == "get" && self.upgrade_get_to_get_mut {
            self.upgrade_get_to_get_mut = false;
            "get_mut"
        } else {
            method
        };

        let obj_str = self.mc_build_method_receiver_string(object, effective_method);
        let method_signature =
            self.mc_resolve_method_call_signature(object, effective_method, arguments);
        let type_name = self.infer_type_name(object);
        let (args, prev_float) = self.mc_build_method_call_arg_strings(
            object,
            effective_method,
            arguments,
            &method_signature,
            type_name,
        );
        self.mc_finalize_method_call_expression(
            object,
            effective_method,
            type_args,
            arguments,
            obj_str,
            args,
            prev_float,
        )
    }
}
