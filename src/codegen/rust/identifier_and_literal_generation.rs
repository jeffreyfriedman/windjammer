//! Identifier and literal expression generation
//!
//! Handles generation of:
//! - Identifier expressions with auto-clone logic
//! - Literal expressions with context-sensitive float inference
//! - Try operator (?)

use crate::parser::{Expression, Literal, Type};

use super::{float_type_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_identifier(
        &mut self,
        name: &str,
        expr_to_generate: &Expression<'ast>,
    ) -> String {
        // Qualified paths use :: from parser (e.g., std::fs::read)
        // Simple identifiers: variable_name -> variable_name
        // Check if this is a struct field and we're in an impl block
        // BUT: Don't apply implicit field access if:
        // 1. It's a parameter name (parameters shadow fields)
        // 2. It's a local variable (local vars shadow fields)
        let is_parameter = self.current_function_params.iter().any(|p| p.name == *name);
        let is_local_variable = self
            .local_variable_scopes
            .iter()
            .any(|scope| scope.contains(name));

        let is_implicit_self_field = self.in_impl_block
            && !is_parameter
            && !is_local_variable
            && self.current_struct_fields.contains(name);
        let base_name = if is_implicit_self_field {
            format!("self.{}", name)
        } else {
            name.to_string()
        };
        let base_name = self.qualify_external_path_identifier(&base_name);
        // Import-driven FQ for stdlib types (`use std::net::Request` → native net path),
        // so associated calls don't lower to a colliding crate-root type (`http::Request`).
        let base_name = if !is_parameter && !is_local_variable {
            self.qualify_stdlib_type_identifier(&base_name)
        } else {
            base_name
        };

        // WDB-367: unit keywords are never binding names — auto_clone cross-function
        // `needs_clone_anywhere("None")` must not yield `None.clone()` (tilemap/graph).
        if name == "None" || name == "true" || name == "false" {
            return if name == "None" {
                "None".to_string()
            } else {
                name.to_string()
            };
        }

        // WDB-347: match-arm bindings are already owned (or `*`-deref'd for `&Copy`).
        // Never auto-clone them — `r.clone()` on Copy f32 payload is noise / tip RED.
        if self.match_arm_bindings.contains(name) {
            return base_name;
        }

        if (self.in_match_arm_needing_string || self.in_owned_value_context)
            && self.module_string_consts.contains(name)
            && !base_name.ends_with(".to_string()")
        {
            return format!("{}.to_string()", base_name);
        }

        // AUTO-CLONE: Check if this variable needs to be cloned at this point
        // CRITICAL: Never clone assignment targets (left side of `=`)
        // DOUBLE-CLONE FIX: Skip auto-clone when inside an explicit .clone() call
        if !self.generating_assignment_target
            && !self.in_explicit_clone_call
            && !self.in_call_argument_generation
            && !self.in_field_access_object
        {
            if self.borrowed_iterator_vars.contains(name)
                && crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                    self.current_function_return_type.as_ref(),
                )
            {
                return base_name;
            }
            if let Some(ref analysis) = self.auto_clone_analysis {
                if analysis
                    .needs_clone(name, self.current_statement_idx)
                    .is_some()
                {
                    // Borrowed/mut-borrowed params don't need cloning:
                    // &T is Copy (reborrow is free), &mut T can be reborrowed.
                    let is_ref_param = self.inferred_borrowed_params.contains(name)
                        || self.inferred_mut_borrowed_params.contains(name);

                    // Skip .clone() for Copy types — they are implicitly copied,
                    // so .clone() is unnecessary noise.
                    let is_copy_type = is_ref_param
                        || analysis.string_literal_vars.contains(name)
                        || self.usize_variables.contains(name)
                        || self
                            .infer_expression_type(expr_to_generate)
                            .as_ref()
                            .is_some_and(|t| {
                                if self.is_type_copy(t) {
                                    return true;
                                }
                                match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        self.is_type_copy(inner)
                                    }
                                    _ => false,
                                }
                            });

                    if !is_copy_type {
                        return format!("{}.clone()", base_name);
                    }
                }
            }

            // &self field clone: when accessing self.field in a &self method,
            // non-Copy types can't be moved out of the reference — auto-clone.
            // Skip in comparison contexts — refs compare fine without cloning.
            if is_implicit_self_field
                && self.inferred_borrowed_params.contains("self")
                && !self.suppress_borrowed_clone
            {
                let field_is_copy = self
                    .current_struct_name
                    .as_ref()
                    .and_then(|sn| self.lookup_struct_field_types(sn.as_str()))
                    .and_then(|fields| fields.get(name))
                    .is_some_and(|ty| self.is_type_copy(ty));
                if !field_is_copy {
                    return format!("{}.clone()", base_name);
                }
            }
        }

        if self.in_owned_value_context
            && !self.in_call_argument_generation
            && !self.generating_assignment_target
            && !self.in_field_access_object
        {
            if let Expression::Identifier { name, .. } = expr_to_generate {
                if let Some(ty) = self.local_var_types.get(name) {
                    if !matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
                        return base_name;
                    }
                }
            }
            if let Some(ty) = self.infer_expression_type(expr_to_generate) {
                if let Type::Reference(inner) | Type::MutableReference(inner) = &ty {
                    if self.is_type_copy(inner) {
                        return format!("*{}", base_name);
                    }
                    return format!("{}.clone()", base_name);
                }
            }
        }

        base_name
    }

    /// Generate code for try operator expression (expr?)
    pub(in crate::codegen::rust) fn generate_try_op(&mut self, expr: &Expression<'ast>) -> String {
        format!("{}?", self.generate_expression(expr))
    }

    pub(in crate::codegen::rust) fn generate_literal_context_sensitive(
        &self,
        lit: &Literal,
    ) -> String {
        // WINDJAMMER PHILOSOPHY: Context-sensitive float type inference
        // The compiler should infer f32 vs f64 based on the surrounding context
        // to avoid ambiguous numeric type errors (Rust E0689)
        match lit {
            Literal::Float(f) => {
                // Priority 1: Struct field type (most specific)
                let float_type = if let (Some(struct_name), Some(field_name)) = (
                    &self.current_struct_literal_name,
                    &self.current_struct_field_name,
                ) {
                    if let Some(fields) = self.lookup_struct_field_types(struct_name) {
                        if let Some(field_type) = fields.get(field_name) {
                            float_type_utilities::extract_float_type_from_context(field_type)
                        } else {
                            "f32"
                        }
                    } else {
                        "f32"
                    }
                } else if let Some(return_type) = &self.current_function_return_type {
                    float_type_utilities::extract_float_type_from_context(return_type)
                } else {
                    "f32"
                };

                let s = f.to_string();
                if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                    format!("{}.0_{}", s, float_type)
                } else {
                    format!("{}_{}", s, float_type)
                }
            }
            // For other literal types, delegate to canonical implementation
            _ => crate::codegen::rust::literals::generate_literal(lit),
        }
    }

    /// Generate literal without expression context (used in older code paths)
    pub(super) fn generate_literal(&self, lit: &Literal) -> String {
        // Delegate to context-sensitive version
        self.generate_literal_context_sensitive(lit)
    }
}
