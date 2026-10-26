//! Method call expression generation
//!
//! Split across `receiver`, `signature_resolution`, `arguments`, and `finalize`.

use crate::parser::*;

mod arguments;
mod finalize;
mod idiom_lowering;
mod receiver;
mod std_idiom_fixups;
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
        if method == crate::parser::optional_chain::OPT_CHAIN {
            return self.unsupported_optional_chain();
        }
        if method == "find" && arguments.len() == 1 {}
        // WJ-LANG-04: `.string()` is the idiomatic Windjammer alias for string conversion.
        let method = if method == "string" {
            "to_string"
        } else {
            method
        };
        // Language-level owned conversion must emit `.to_string()` before IR Borrow
        // (`push_str(&self.rows)` is invalid). Not an ownership oracle — syntax sugar.
        if crate::type_classification::is_language_level_owned_string_convert(method)
            && arguments.is_empty()
        {
            let obj_str = self.mc_build_method_receiver_string(object, method);
            let base = obj_str
                .trim_end_matches(".to_string()")
                .trim_end_matches(".to_owned()");
            // `s[a..].to_string()`: the desugared slice is `&s[a..]`; the conversion applies to the slice itself.
            let base = if matches!(object, Expression::MethodCall { method: m, arguments: a, .. } if m == "slice" && a.len() == 2) {
                base.strip_prefix('&').unwrap_or(base)
            } else {
                base
            };
            return format!("{base}.to_string()");
        }
        // `.as_str()` is identity in Windjammer (W0012): lower exactly like the receiver alone.
        if method == "as_str" && arguments.is_empty() {
            return self.generate_expression(object);
        }
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
        // ADR-0018: surface collection methods (`group_by`, ...) call the inlined enumerable runtime.
        let effective_method = self
            .enumerable_rust_name(receiver_type_name.as_deref(), effective_method, arguments.len())
            .unwrap_or(effective_method);

        let obj_str = self.mc_build_method_receiver_string(object, effective_method);
        // `reader.read_line(line)` / `read_to_string(buf)`: std fills a `&mut String`. The lint-clean spelling has no
        // `&mut`, so a bare identifier argument is passed by mutable borrow, never cloned (a clone is filled and the
        // caller's buffer stays empty, so read loops never end).
        // A module receiver (`fs.read_to_string(path)`) is a path call, not a reader method.
        let receiver_is_module = matches!(object, Expression::Identifier { name, .. } if self.identifier_is_static_call_root(name));
        if !receiver_is_module && matches!(effective_method, "read_line" | "read_to_string") && arguments.len() == 1 {
            if let Expression::Identifier { name, .. } = arguments[0].1 {
                return format!("{obj_str}.{effective_method}(&mut {name})");
            }
        }
        let method_signature =
            self.mc_resolve_method_call_signature(object, effective_method, arguments);
        let type_name = self
            .self_field_access_receiver_type_name(object)
            .or_else(|| self.mc_infer_method_receiver_type_name(object))
            .or_else(|| self.infer_type_name(object))
            .or_else(|| {
                self.infer_expression_type(object)
                    .and_then(|t| Self::type_to_name(&t))
            });
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
