//! Method call receiver codegen (object expr + recv fixes).

use crate::parser::Expression;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn mc_build_method_receiver_string(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
    ) -> String {
        // METHOD CALL CONTEXT: Suppress Vec index auto-clone when generating the
        // object of a method call. Methods take &self or &mut self, so Rust allows
        // calling methods on &T returned by Vec indexing without cloning.
        // e.g., self.lights[i].is_enabled() → no need to clone the whole Light2D
        let prev_field_access = self.in_field_access_object;
        self.in_field_access_object = true;
        let prev_explicit_clone = self.in_explicit_clone_call;
        let prev_coerce = self.coerce_string_literals_to_owned;

        // Type-preserving methods (registry: return `Self`) already own/copy the
        // value — suppress nested auto-clone and string-literal `.into()` coercion.
        let recv_ty_name = self
            .infer_expression_type(object)
            .as_ref()
            .and_then(Self::type_to_name);
        let registry = self
            .global_signature_registry()
            .unwrap_or(&self.signature_registry);
        let is_type_preserving =
            crate::codegen::rust::stdlib_method_traits::method_is_type_preserving_qualified(
                method,
                recv_ty_name.as_deref(),
                registry,
            );
        // WJ `string` / Rust `to_string` are language-level owned conversions (not registry Self).
        let is_explicit_owned_convert =
            crate::type_classification::is_language_level_owned_string_convert(method);
        if is_type_preserving || is_explicit_owned_convert {
            self.in_explicit_clone_call = true;
            self.coerce_string_literals_to_owned = false;
        }
        let mut obj_str = self.generate_expression_with_precedence(object);
        self.coerce_string_literals_to_owned = prev_coerce;
        self.in_field_access_object = prev_field_access;
        self.in_explicit_clone_call = prev_explicit_clone;
        // E0507: `collection[i].method(args)` on non-Copy elements must clone before the call
        // only when the method consumes an owned receiver. Borrowed receivers (`&self`) may
        // call through the index expression directly (signature-driven, not always-clone).
        if matches!(object, Expression::Index { .. }) && !obj_str.ends_with(".clone()") {
            if let Some(recv_ty) = self.infer_expression_type(object) {
                if !self.is_type_copy(&recv_ty) {
                    let mut needs_clone = false;
                    if let Some(tn) = Self::type_to_name(&recv_ty) {
                        let qualified = format!("{tn}::{method}");
                        let sig_opt = self
                            .get_signature_with_global(&qualified)
                            .or_else(|| self.signature_registry.get_signature(&qualified))
                            .or_else(|| {
                                let base = tn.split('<').next().unwrap_or(&tn);
                                self.get_signature_with_global(&format!("{base}::{method}"))
                            });
                        needs_clone = sig_opt.is_some_and(|sig| {
                            if !sig.has_self_receiver {
                                return false;
                            }
                            let base = tn.split('<').next().unwrap_or(tn.as_str());
                            let ownership_recv =
                                self.effective_method_self_ownership(&qualified, sig);
                            let ownership_base = self
                                .effective_method_self_ownership(&format!("{base}::{method}"), sig);
                            let ownership = match (ownership_recv, ownership_base) {
                                (crate::analyzer::OwnershipMode::MutBorrowed, _)
                                | (_, crate::analyzer::OwnershipMode::MutBorrowed) => {
                                    crate::analyzer::OwnershipMode::MutBorrowed
                                }
                                (crate::analyzer::OwnershipMode::Borrowed, _)
                                | (_, crate::analyzer::OwnershipMode::Borrowed) => {
                                    crate::analyzer::OwnershipMode::Borrowed
                                }
                                (other, _) => other,
                            };
                            !matches!(
                                ownership,
                                crate::analyzer::OwnershipMode::Borrowed
                                    | crate::analyzer::OwnershipMode::MutBorrowed
                            ) && self.method_requires_consuming_self_receiver(&qualified, sig)
                        });
                    }
                    if needs_clone {
                        obj_str = format!("{}.clone()", obj_str);
                    }
                }
            }
        }

        // E0507: `borrowed_var.method(args)` when the method consumes `self` (owned receiver)
        // and the variable is a borrowed iterator variable (from `for x in &collection`).
        // Must clone: `condition.clone().evaluate(state)` instead of `condition.evaluate(state)`.
        if let Expression::Identifier { name, .. } = object {
            // Skip consume-clone for type-preserving methods (already computed above).
            let is_borrowed_iter =
                self.borrowed_iterator_vars.contains(name) && !is_type_preserving;
            let is_mut_borrowed_param =
                self.inferred_mut_borrowed_params.contains(name) && !is_type_preserving;
            let is_shared_borrowed_param =
                self.inferred_borrowed_params.contains(name) && !is_type_preserving;
            if is_borrowed_iter || is_mut_borrowed_param || is_shared_borrowed_param {
                let qualified_from_self = (name == "self")
                    .then(|| {
                        self.current_struct_name
                            .as_ref()
                            .map(|sn| format!("{sn}::{method}"))
                    })
                    .flatten();
                if let Some(recv_ty) = self.infer_expression_type(object) {
                    if !self.is_type_copy(&recv_ty) {
                        let sig_opt = if let Some(ref qualified) = qualified_from_self {
                            self.get_signature_with_global(qualified)
                                .or_else(|| self.signature_registry.get_signature(qualified))
                        } else if let Some(tn) = Self::type_to_name(&recv_ty) {
                            let qualified = format!("{}::{}", tn, method);
                            self.signature_registry
                                .get_signature(&qualified)
                                .or_else(|| {
                                    let base = tn.split('<').next().unwrap_or(&tn);
                                    if base != tn {
                                        let base_q = format!("{base}::{method}");
                                        self.get_signature_with_global(&base_q)
                                    } else {
                                        None
                                    }
                                })
                                .or_else(|| {
                                    Self::extract_dyn_trait_name(&recv_ty).and_then(|trait_name| {
                                        let trait_q = format!("{trait_name}::{method}");
                                        self.get_signature_with_global(&trait_q)
                                    })
                                })
                                .or_else(|| {
                                    self.signature_registry
                                        .find_signature_ending_with(&format!("::{method}"))
                                })
                        } else {
                            None
                        };
                        if let Some(sig) = sig_opt {
                            let qualified = qualified_from_self.unwrap_or_else(|| {
                                Self::type_to_name(&recv_ty)
                                    .map(|tn| format!("{tn}::{method}"))
                                    .unwrap_or_else(|| method.to_string())
                            });
                            let callee_consumes_self = name == "self"
                                && self.current_impl_consuming_self_methods.contains(method);
                            if sig.has_self_receiver
                                && (callee_consumes_self
                                    || self
                                        .method_requires_consuming_self_receiver(&qualified, sig))
                                && !obj_str.ends_with(".clone()")
                            {
                                // From `&mut self`, calling another `&mut self` method
                                // must reborrow — never `self.clone().method()`.
                                let ownership =
                                    self.effective_method_self_ownership(&qualified, sig);
                                let skip_clone = is_mut_borrowed_param
                                    && matches!(
                                        ownership,
                                        crate::analyzer::OwnershipMode::MutBorrowed
                                            | crate::analyzer::OwnershipMode::Borrowed
                                    );
                                let skip_owned_mut_helper = is_mut_borrowed_param
                                    && matches!(ownership, crate::analyzer::OwnershipMode::Owned)
                                    && !matches!(
                                        sig.return_type.as_ref(),
                                        Some(crate::parser::Type::Custom(ret))
                                            if self.current_struct_name.as_ref().is_some_and(|sn| {
                                                sn == ret
                                                    || sn.split('<').next().unwrap_or(sn) == ret
                                            })
                                    );
                                if !skip_clone && !skip_owned_mut_helper {
                                    obj_str = format!("{}.clone()", obj_str);
                                }
                            }
                        } else if is_borrowed_iter
                            && !is_mut_borrowed_param
                            && !is_shared_borrowed_param
                            && !obj_str.ends_with(".clone()")
                        {
                            obj_str = format!("{}.clone()", obj_str);
                        }
                    }
                }
            }
        }

        // DOUBLE-CLONE SAFETY NET: If the object was auto-cloned by the FieldAccess
        // handler and this IS a language-level `.clone()` call, strip the redundant auto-clone.
        // e.g., "stack.item.clone()" from auto-clone + ".clone()" from source
        //     → should be "stack.item.clone()", not "stack.item.clone().clone()"
        crate::codegen::rust::string_utilities::strip_redundant_auto_clone_before_explicit_clone(
            &mut obj_str,
            method,
        );

        // Option methods that take owned `self` (unwrap, …) on a borrowed path need
        // `.as_ref()` first — driven by Option::{method} ownership in the registry.
        if crate::codegen::rust::stdlib_method_traits::option_owned_self_method(
            method,
            &self.signature_registry,
        ) && self.expression_traces_to_inferred_borrowed_param(object)
            && !obj_str.contains(".as_ref()")
            && !obj_str.contains(".clone()")
        {
            obj_str = format!("{}.as_ref()", obj_str);
        }

        // E0507: borrowed `Option` adapters that Rust implements by-value
        // (`map` / `and_then` / …) lower via `.as_ref()` so `&T` methods stay borrowed.
        // Driven by Option stdlib_meta + method registry — not a bare method-name list.
        if self.expression_traces_to_inferred_borrowed_param(object)
            && !obj_str.contains(".as_ref()")
            && crate::codegen::rust::stdlib_method_traits::option_adapter_needs_as_ref(
                method,
                &self.signature_registry,
            )
        {
            obj_str = format!("{}.as_ref()", obj_str);
        }

        // E0507: `self.nested.field.method()` on borrowed self when method consumes owned receiver.
        if self.codegen_expression_traces_to_self(object)
            && (self.inferred_mut_borrowed_params.contains("self")
                || self.inferred_borrowed_params.contains("self"))
            && !obj_str.ends_with(".clone()")
        {
            if let Some(recv_ty) = self.infer_expression_type(object) {
                if !self.is_type_copy(&recv_ty) {
                    let mut needs_clone = false;
                    if let Some(tn) = Self::type_to_name(&recv_ty) {
                        let qualified = format!("{}::{}", tn, method);
                        let sig_opt =
                            self.signature_registry
                                .get_signature(&qualified)
                                .or_else(|| {
                                    let base = tn.split('<').next().unwrap_or(&tn);
                                    self.get_signature_with_global(&format!("{base}::{method}"))
                                });
                        needs_clone = sig_opt.is_some_and(|sig| {
                            if !sig.has_self_receiver {
                                return false;
                            }
                            // Look up ownership on the *receiver* type
                            // (`SimNetwork::poll`), not the enclosing impl
                            // (`SimHarness::poll`) — otherwise upgrades miss.
                            let recv_qualified = format!("{tn}::{method}");
                            let base = tn.split('<').next().unwrap_or(tn.as_str());
                            let base_qualified = format!("{base}::{method}");
                            let ownership_recv =
                                self.effective_method_self_ownership(&recv_qualified, sig);
                            let ownership_base =
                                self.effective_method_self_ownership(&base_qualified, sig);
                            let ownership = match (ownership_recv, ownership_base) {
                                (crate::analyzer::OwnershipMode::MutBorrowed, _)
                                | (_, crate::analyzer::OwnershipMode::MutBorrowed) => {
                                    crate::analyzer::OwnershipMode::MutBorrowed
                                }
                                (crate::analyzer::OwnershipMode::Borrowed, _)
                                | (_, crate::analyzer::OwnershipMode::Borrowed) => {
                                    crate::analyzer::OwnershipMode::Borrowed
                                }
                                (other, _) => other,
                            };
                            let enclosing_mut = self.inferred_mut_borrowed_params.contains("self");
                            match ownership {
                                crate::analyzer::OwnershipMode::Borrowed
                                | crate::analyzer::OwnershipMode::MutBorrowed => false,
                                crate::analyzer::OwnershipMode::Owned if enclosing_mut => {
                                    // Analyzer may mark mutating methods Owned (field
                                    // move into a helper). From `&mut self`, prefer
                                    // reborrowing `self.field` unless the method
                                    // returns the receiver type (builder / consume).
                                    let returns_recv_self = matches!(
                                        sig.return_type.as_ref(),
                                        Some(crate::parser::Type::Custom(name))
                                            if name == &tn || name == base
                                    );
                                    returns_recv_self
                                }
                                crate::analyzer::OwnershipMode::Owned => true,
                            }
                        });
                    }
                    if needs_clone {
                        obj_str = format!("{}.clone()", obj_str);
                    }
                }
            }
        }

        obj_str
    }

    /// True when `expr` is a bare borrowed param or a field/index chain rooted at one.
    fn expression_traces_to_inferred_borrowed_param(&self, expr: &Expression<'ast>) -> bool {
        match expr {
            Expression::Identifier { name, .. } => self.inferred_borrowed_params.contains(name),
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                self.expression_traces_to_inferred_borrowed_param(object)
            }
            _ => false,
        }
    }

    /// Extract the trait name from `Box<dyn Trait>`, `dyn Trait`, or `TraitObject("Trait")`.
    fn extract_dyn_trait_name(ty: &crate::parser::Type) -> Option<&str> {
        use crate::parser::Type;
        match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::extract_dyn_trait_name(inner)
            }
            Type::Parameterized(name, params) if name == "Box" => params
                .first()
                .and_then(|inner| Self::extract_dyn_trait_name(inner)),
            Type::TraitObject(name) => Some(name.as_str()),
            Type::Custom(name) => name.strip_prefix("dyn "),
            _ => None,
        }
    }
}
