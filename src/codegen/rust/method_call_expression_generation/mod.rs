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
        if method == "find" && arguments.len() == 1 {
        }
        // WJ-LANG-04: `.string()` is the idiomatic Windjammer alias for string conversion.
        let method = if method == "string" { "to_string" } else { method };
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
                        let base = receiver_type.split('<').next().unwrap_or(&receiver_type);
                        let trivial_key = format!("{base}::{method}");
                        let is_trivial_accessor =
                            self.trivial_copy_field_accessors.contains(&trivial_key)
                                || self
                                    .trivial_copy_field_accessors
                                    .contains(&format!("{receiver_type}::{method}"));
                        if recv_is_ref || !has_method || is_trivial_accessor {
                            return format!("{}.{}", self.generate_expression(object), method);
                        }
                    }
                }
            }
        }

        // TDD FIX: Upgrade map shared-get to get_mut when the bound value is mutated downstream.
        // Signature-gated: only map key lookups returning Option<&V>, never arbitrary `.get`.
        let receiver_type_name = self
            .infer_type_name(object)
            .or_else(|| self.infer_indexed_element_type_name(object));
        let mut_sibling: Option<String> = if self.upgrade_get_to_get_mut
            && crate::codegen::rust::stdlib_method_traits::is_map_shared_get_call(
                method,
                receiver_type_name.as_deref(),
                &self.signature_registry,
            ) {
            crate::codegen::rust::stdlib_method_traits::map_option_mut_ref_method_name(
                receiver_type_name.as_deref(),
                &self.signature_registry,
            )
            .map(|s| s.to_string())
        } else {
            None
        };
        if self.upgrade_get_to_get_mut {
            self.upgrade_get_to_get_mut = false;
        }
        let effective_method = mut_sibling.as_deref().unwrap_or(method);

        
        let obj_str = self.mc_build_method_receiver_string(object, effective_method);
        let method_signature =
            self.mc_resolve_method_call_signature(object, effective_method, arguments);
        let type_name = self
            .mc_infer_method_receiver_type_name(object)
            .or_else(|| self.infer_type_name(object));
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
            &method_signature,
            obj_str,
            args,
            prev_float,
        )
    }
}
