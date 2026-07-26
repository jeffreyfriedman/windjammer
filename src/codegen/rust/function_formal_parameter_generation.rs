//! Formal parameter list emission (excluding implicit `self` receiver).

use std::collections::HashSet;

use crate::analyzer::*;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn compute_unused_formal_parameter_names(
        &self,
        func: &FunctionDecl<'ast>,
    ) -> HashSet<String> {
        let body_refs: Vec<&Statement> = func.body.to_vec();
        func.parameters
            .iter()
            .filter(|p| p.name != "self")
            .filter(|p| {
                let used_in_body = Self::variable_used_in_statements(&body_refs, &p.name);
                let used_in_decorators = func.decorators.iter().any(|d| {
                    d.arguments
                        .iter()
                        .any(|(_, expr)| Self::variable_used_in_expression(expr, &p.name))
                });
                !used_in_body && !used_in_decorators
            })
            .map(|p| p.name.clone())
            .collect()
    }

    pub(in crate::codegen::rust) fn refresh_unused_let_bindings_for_function_body(
        &mut self,
        body: &[&'ast Statement<'ast>],
    ) {
        // TDD FIX: Pre-compute unused let bindings and for-loop variables.
        // Like unused params, these get prefixed with `_` in the generated Rust.
        self.unused_let_bindings.clear();
        Self::find_unused_bindings(body, &mut self.unused_let_bindings);
    }

    pub(in crate::codegen::rust) fn collect_additional_formal_parameter_strings(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
        func: &FunctionDecl<'ast>,
        needs_lifetime: bool,
        unused_params: &HashSet<String>,
    ) -> Vec<String> {
        let body_modifies = if self.in_trait_impl {
            self.function_modifies_self(&analyzed.decl)
        } else {
            self.function_modifies_self_or_derived(&analyzed.decl)
        };
        func.parameters
            .iter()
            .enumerate()
            .map(|(param_idx, param)| {
                // SMART STRING INFERENCE: Use the inferred type from analyzer (string → &str vs String)
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(param_idx)
                    .unwrap_or(&param.type_);

                // E0053: Trait impl formal parameters must match the trait item. Plain `string` in
                // source is owned `String` — do not emit `&str` from str_ref inference on the impl.
                let is_owned_string_decl = matches!(&param.type_, Type::String)
                    || matches!(&param.type_, Type::Custom(name) if name == "string");

                let converged_analyzer_borrow = matches!(
                    analyzed.inferred_ownership.get(&param.name),
                    Some(OwnershipMode::Borrowed)
                );
                if param.name != "self"
                    && !self.in_trait_impl
                    && !analyzed.field_extract_parameters.contains(&param.name)
                    && !self.param_moves_via_struct_literal_init(func.body.as_slice(), &param.name)
                    && !self.is_type_copy(&param.type_)
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.func_is_pure_forwarding_delegate(func)
                    && !self.param_passed_from_multiple_statements(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.param_has_forward_ref_keep_owned(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.current_fn_mixed_forwarder_params.contains(&param.name)
                    && !self.param_passes_to_wj_owned_sibling_call(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.current_struct_name.as_ref().is_some_and(|sn| {
                        self.struct_is_owned_engine_key_facade(sn, param)
                    })
                    && !self.is_collection_key_owned_param(param, func)
                    && !self.param_only_used_in_discarding_let_binding(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && (self.param_should_emit_borrowed_delegation_formal(param, func)
                        || (converged_analyzer_borrow
                            && (crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                                || self.is_type_copy(&param.type_)))
                        || (self.inferred_borrowed_params.contains(&param.name)
                            && !self.param_is_single_arg_call_only_delegate(param, func)
                            && !self.param_passed_from_multiple_statements(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            )))
                {
                    let type_str =
                        self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                    self.emitted_rust_ref_formals.insert(param.name.clone());
                    self.inferred_borrowed_params.insert(param.name.clone());
                    self.inferred_mut_borrowed_params.remove(&param.name);
                    return format!("{}: {}", param.name, type_str);
                }

                let formal_type: &Type = if self.in_trait_impl
                    && param.name != "self"
                    && is_owned_string_decl
                {
                    &param.type_
                } else if param.name != "self"
                    && self.is_collection_key_owned_param(param, func)
                {
                    &param.type_
                } else if param.name != "self"
                    && matches!(inferred_type, Type::Reference(_))
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && self.is_type_copy(&param.type_)
                    && !self.inferred_borrowed_params.contains(&param.name)
                {
                    // Copy aggregate: spurious Reference from ref analysis — emit by-value
                    // unless field-enum-match kept an active borrow (bug_copy_vec3).
                    &param.type_
                } else if param.name != "self"
                    && matches!(inferred_type, Type::Reference(_))
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.is_type_copy(&param.type_)
                    && self.param_passed_to_owned_self_method_arg(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                {
                    &param.type_
                } else if param.name != "self"
                    && matches!(inferred_type, Type::Reference(_))
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.is_type_copy(&param.type_)
                    && self.param_only_used_in_discarding_let_binding(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                {
                    &param.type_
                } else if param.name != "self"
                    && matches!(inferred_type, Type::Reference(_))
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.is_type_copy(&param.type_)
                    && (self.param_only_forwards_to_emitted_owned_callees(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ) || self.current_struct_name.as_ref().is_some_and(|sn| {
                        self.struct_is_owned_engine_key_facade(sn, param)
                    }))
                {
                    &param.type_
                } else if param.name != "self"
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.is_type_copy(&param.type_)
                    && (self.param_has_forward_ref_keep_owned(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ) || self.current_fn_mixed_forwarder_params.contains(&param.name)
                    || self.param_passed_from_multiple_statements(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ))
                {
                    &param.type_
                } else {
                    inferred_type
                };
                let trait_impl_owned_string =
                    self.in_trait_impl && param.name != "self" && is_owned_string_decl;

                // PHASE 9 OPTIMIZATION: Check if this parameter should use Cow<'_, T>
                if self.cow_optimizations.contains(&param.name) {
                    let base_type = self.type_to_rust(formal_type);
                    // For String types, use Cow<'_, str>
                    let cow_type = if base_type == "String" {
                        "Cow<'_, str>".to_string()
                    } else {
                        format!("Cow<'_, {}>", base_type)
                    };
                    return format!("{}: {}", param.name, cow_type);
                }

                if param.name == "self"
                    && !self.in_trait_impl
                    && !super::self_analysis::function_calls_owned_self_method(
                        &analyzed.decl,
                        &self.signature_registry,
                        self.current_struct_name.as_deref(),
                    )
                    && !matches!(
                        self.get_effective_self_ownership(&func.name, analyzed),
                        Some(OwnershipMode::Owned)
                    )
                    && self.function_calls_self_with_recorded_receiver(
                        &analyzed.decl,
                        OwnershipMode::MutBorrowed,
                    )
                {
                    self.inferred_mut_borrowed_params.insert("self".to_string());
                    self.inferred_borrowed_params.remove("self");
                    self.record_self_receiver_upgrade(
                        &func.name,
                        self.get_param_ownership("self", analyzed),
                        "&mut self",
                    );
                    return "&mut self".to_string();
                }

                // Self-delegation fallback: if the fixed-point pre-pass recorded this
                // method as MutBorrowed (via transitive delegation), use &mut self.
                if param.name == "self" && !self.in_trait_impl {
                    if let Some(sn) = self.current_struct_name.as_deref() {
                        let qualified = format!("{}::{}", sn, func.name);
                        if self.self_receiver_upgrades.get(&qualified)
                            == Some(&OwnershipMode::MutBorrowed)
                        {
                            self.inferred_mut_borrowed_params.insert("self".to_string());
                            self.inferred_borrowed_params.remove("self");
                            return "&mut self".to_string();
                        }
                    }
                }

                // Handle explicit ownership hints (self, &self, &mut self)
                let mut type_str = match &param.ownership {
                    OwnershipHint::Owned => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
                            let consumes_self = super::self_analysis::function_consumes_self(&analyzed.decl)
                                || super::self_analysis::function_return_moves_self_fields(&analyzed.decl);
                            let eff_ownership =
                                self.get_effective_self_ownership(&func.name, analyzed);
                            let self_str = if let Some(ownership_mode) = eff_ownership {
                                match ownership_mode {
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        if !self.in_trait_impl
                                            && (self.method_returns_impl_struct(func) || consumes_self) =>
                                    {
                                        if body_modifies { "mut self" } else { "self" }
                                    }
                                    OwnershipMode::MutBorrowed => "&mut self",
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            "&mut self"
                                        } else if !self.in_trait_impl
                                            && self.method_returns_impl_struct(func)
                                        {
                                            if body_modifies {
                                                "mut self"
                                            } else {
                                                self.owned_self_receiver(&analyzed.decl)
                                            }
                                        } else {
                                            "&self"
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        if self.in_trait_impl {
                                            if body_modifies {
                                                "mut self"
                                            } else {
                                                "self"
                                            }
                                        } else {
                                            self.owned_self_receiver(&analyzed.decl)
                                        }
                                    }
                                }
                            } else {
                                if self.in_trait_impl {
                                    "self"
                                } else {
                                    self.owned_self_receiver(&analyzed.decl)
                                }
                            };
                            // Sync borrowed-params sets with actual generated receiver.
                            match self_str {
                                "&self" => {
                                    self.inferred_borrowed_params.insert("self".to_string());
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                                "&mut self" => {
                                    self.inferred_mut_borrowed_params.insert("self".to_string());
                                    self.inferred_borrowed_params.remove("self");
                                }
                                _ => {
                                    self.inferred_borrowed_params.remove("self");
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                            }
                            self.record_self_receiver_upgrade(
                                &func.name,
                                eff_ownership,
                                self_str,
                            );
                            return self_str.to_string();
                        }
                        // Check if the analyzer inferred MutBorrowed (e.g., Copy type
                        // mutated through method call — caller wants mutation visible).
                        // Skip in trait impls: trait signature must match exactly.
                        if !self.in_trait_impl {
                            if !param.is_mutable {
                                if let Some(OwnershipMode::MutBorrowed) =
                                    self.get_param_ownership(&param.name, analyzed)
                                {
                                    self.inferred_mut_borrowed_params
                                        .insert(param.name.clone());
                                    return format!(
                                        "&mut {}",
                                        self.type_to_rust(formal_type)
                                    );
                                }
                            }
                            if !analyzed.returned_parameters.contains(&param.name)
                                && (!param.is_mutable
                                    || analyzed
                                        .field_mutated_parameters
                                        .contains(&param.name))
                                && analyzed.mutated_parameters.contains(&param.name)
                                && !self.is_type_copy(formal_type)
                            {
                                self.inferred_mut_borrowed_params
                                    .insert(param.name.clone());
                                return format!(
                                    "&mut {}",
                                    self.type_to_rust(formal_type)
                                );
                            }
                        }
                        if param.name != "self"
                            && !self.param_moves_via_struct_literal_init(
                                func.body.as_slice(),
                                &param.name,
                            )
                            && self.param_should_emit_borrowed_delegation_formal(param, func)
                            && !self.is_collection_key_owned_param(param, func)
                        {
                            let type_str =
                                self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                            self.emitted_rust_ref_formals.insert(param.name.clone());
                            self.inferred_borrowed_params.insert(param.name.clone());
                            self.inferred_mut_borrowed_params.remove(&param.name);
                            return format!("{}: {}", param.name, type_str);
                        }
                        // Owned parameters are always mutable in Windjammer
                        return format!("mut {}: {}", param.name, self.type_to_rust(formal_type));
                    }
                    OwnershipHint::Ref => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
                            if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            return "&mut self".to_string();
                                        }
                                        return "&self".to_string();
                                    }
                                    OwnershipMode::Owned => {
                                        return "self".to_string();
                                    }
                                }
                            }
                            if !self.in_trait_impl && body_modifies {
                                return "&mut self".to_string();
                            }
                            return "&self".to_string();
                        }
                        // Don't add & if the type is already a Reference
                        if matches!(
                            formal_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            if self.is_type_copy(&param.type_)
                                && !self.inferred_borrowed_params.contains(&param.name)
                                && (!self.ir_cutover.ownership
                                    || !matches!(
                                        self.get_param_ownership(&param.name, analyzed),
                                        Some(OwnershipMode::Borrowed)
                                    ))
                            {
                                self.type_to_rust(&param.type_)
                            } else {
                                self.type_to_rust(formal_type)
                            }
                        } else {
                            // TDD FIX: Copy types pass by value even with Ref hint
                            if self.is_type_copy(formal_type) {
                                self.type_to_rust(formal_type)
                            } else {
                                // TDD FIX: Borrowed → &T (including &String for strings)
                                // Correctness > idioms: &String works with Vec<String> methods
                                format!("&{}", self.type_to_rust(formal_type))
                            }
                        }
                    }
                    OwnershipHint::Mut => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
                            if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                return match ownership_mode {
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            "&mut self".to_string()
                                        } else {
                                            "&self".to_string()
                                        }
                                    }
                                    OwnershipMode::MutBorrowed => "&mut self".to_string(),
                                    OwnershipMode::Owned => {
                                        if self.in_trait_impl {
                                            "self".to_string()
                                        } else {
                                            self.owned_self_receiver(&analyzed.decl).to_string()
                                        }
                                    }
                                };
                            }
                            return "&mut self".to_string();
                        }
                        // Don't add &mut if the type is already a MutableReference
                        if matches!(formal_type, Type::MutableReference(_)) {
                            self.type_to_rust(formal_type)
                        } else {
                            format!("&mut {}", self.type_to_rust(formal_type))
                        }
                    }
                    OwnershipHint::Inferred => {
                        if param.name == "self" {
                            if !self.in_trait_impl
                                && super::self_analysis::function_calls_owned_self_method(
                                    &analyzed.decl,
                                    &self.signature_registry,
                                    self.current_struct_name.as_deref(),
                                )
                            {
                                let body_modifies = body_modifies;
                                let self_str = if body_modifies
                                    && self.method_returns_impl_struct(func)
                                {
                                    "mut self"
                                } else {
                                    "self"
                                };
                                self.inferred_borrowed_params.remove("self");
                                self.inferred_mut_borrowed_params.remove("self");
                                return self_str.to_string();
                            }
                            if !self.in_trait_impl
                                && !matches!(param.ownership, OwnershipHint::Owned)
                                && matches!(
                                    analyzed.inferred_ownership.get("self"),
                                    Some(OwnershipMode::Borrowed)
                                )
                                && !self.method_returns_impl_struct(func)
                                && !super::self_analysis::function_calls_owned_self_method(
                                    &analyzed.decl,
                                    &self.signature_registry,
                                    self.current_struct_name.as_deref(),
                                )
                            {
                                self.inferred_borrowed_params.insert("self".to_string());
                                self.inferred_mut_borrowed_params.remove("self");
                                return "&self".to_string();
                            }
                            if !self.in_trait_impl
                                && matches!(
                                    analyzed.inferred_ownership.get("self"),
                                    Some(OwnershipMode::Owned)
                                )
                                && (super::self_analysis::function_calls_owned_self_method(
                                    &analyzed.decl,
                                    &self.signature_registry,
                                    self.current_struct_name.as_deref(),
                                ) || super::self_analysis::function_consumes_self(&analyzed.decl))
                            {
                                let body_modifies = body_modifies;
                                let self_str = if body_modifies
                                    && self.method_returns_impl_struct(func)
                                {
                                    "mut self"
                                } else {
                                    self.owned_self_receiver(&analyzed.decl)
                                };
                                self.inferred_borrowed_params.remove("self");
                                self.inferred_mut_borrowed_params.remove("self");
                                return self_str.to_string();
                            }
                            let body_modifies = body_modifies;
                            let returns_self = self.method_returns_impl_struct(&analyzed.decl);
                            let consumes_self = super::self_analysis::function_consumes_self(&analyzed.decl)
                                || super::self_analysis::function_return_moves_self_fields(&analyzed.decl)
                                || super::self_analysis::function_calls_owned_self_method(
                                    &analyzed.decl,
                                    &self.signature_registry,
                                    self.current_struct_name.as_deref(),
                                );
                            let self_str = if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                match ownership_mode {
                                    OwnershipMode::Owned => {
                                        if self.in_trait_impl {
                                            "self"
                                        } else if body_modifies {
                                            "mut self"
                                        } else {
                                            self.owned_self_receiver(&analyzed.decl)
                                        }
                                    }
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        if !self.in_trait_impl && (returns_self || consumes_self) =>
                                    {
                                        if body_modifies { "mut self" } else { "self" }
                                    }
                                    OwnershipMode::MutBorrowed
                                        if consumes_self && !body_modifies =>
                                    {
                                        "self"
                                    }
                                    OwnershipMode::MutBorrowed => "&mut self",
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            "&mut self"
                                        } else {
                                            "&self"
                                        }
                                    }
                                }
                            } else if body_modifies && returns_self {
                                "mut self"
                            } else if consumes_self {
                                "self"
                            } else if body_modifies {
                                "&mut self"
                            } else if returns_self {
                                "self"
                            } else {
                                "&self"
                            };
                            // Sync borrowed-params sets with actual generated receiver.
                            match self_str {
                                "&self" => {
                                    self.inferred_borrowed_params.insert("self".to_string());
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                                "&mut self" => {
                                    self.inferred_mut_borrowed_params.insert("self".to_string());
                                    self.inferred_borrowed_params.remove("self");
                                }
                                _ => {
                                    self.inferred_borrowed_params.remove("self");
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                            }
                            let eff = self.get_effective_self_ownership(&func.name, analyzed);
                            self.record_self_receiver_upgrade(
                                &func.name,
                                eff,
                                self_str,
                            );
                            return self_str.to_string();
                        }

                        // Pure delegation / call-only forwarders: emit &T even when IR/analyzer
                        // left the converged formal as owned (wdb LsmEngine::get → MemoryEngine::get).
                        if self.param_should_emit_borrowed_delegation_formal(param, func)
                            && !self.param_moves_via_struct_literal_init(
                                func.body.as_slice(),
                                &param.name,
                            )
                            && !self.is_collection_key_owned_param(param, func)
                            && !self.param_passed_from_multiple_statements(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            )
                            && !self.func_is_pure_forwarding_delegate(func)
                        {
                            let type_str =
                                self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                            self.emitted_rust_ref_formals.insert(param.name.clone());
                            self.inferred_borrowed_params.insert(param.name.clone());
                            self.inferred_mut_borrowed_params.remove(&param.name);
                            return format!("{}: {}", param.name, type_str);
                        }

                        // Check if type already has ownership baked in (like &str from string inference)
                        let force_owned_collection_key =
                    self.is_collection_key_owned_param(param, func);
                        let discard_only = self.param_only_used_in_discarding_let_binding(
                            func.body.as_slice(),
                            &param.name,
                            func,
                        );
                        if matches!(
                            formal_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) && !force_owned_collection_key
                            && !discard_only
                        {
                            if self.is_type_copy(&param.type_)
                                && !self.inferred_borrowed_params.contains(&param.name)
                                && (!self.ir_cutover.ownership
                                    || !matches!(
                                        self.get_param_ownership(&param.name, analyzed),
                                        Some(OwnershipMode::Borrowed)
                                    ))
                            {
                                self.type_to_rust(&param.type_)
                            } else {
                                // Already has & or &mut - just convert
                                self.type_to_rust(formal_type)
                            }
                        } else if force_owned_collection_key {
                            self.type_to_rust(&param.type_)
                        } else {
                            // Apply ownership from IR solver (or legacy analyzer when cutover off).
                            let registry_ownership = self
                                .get_signature_with_global(&func.name)
                                .and_then(|sig| sig.param_ownership.get(param_idx).copied());
                            let mut ownership_mode = self
                                .get_param_ownership(&param.name, analyzed)
                                .or_else(|| {
                                    analyzed.inferred_ownership.get(&param.name).copied()
                                })
                                .or(registry_ownership)
                                .unwrap_or(OwnershipMode::Owned);

                            if self.current_struct_name.as_ref().is_some_and(|sn| {
                                self.struct_is_owned_engine_key_facade(sn, param)
                            }) {
                                ownership_mode = OwnershipMode::Owned;
                            }

                            // Field-extract returns (`key.bytes` / `msg.payload`) need an owned
                            // formal so the body can move the field; call sites clone when the
                            // binding is reused (WDB-044/045).
                            if analyzed.field_extract_parameters.contains(&param.name)
                                && !self.is_type_copy(&param.type_)
                            {
                                ownership_mode = OwnershipMode::Owned;
                            }

                            if self.param_used_in_if_with_condition_and_branches(
                                func.body.as_slice(),
                                &param.name,
                            ) || self.param_used_in_if_else_both_branches(
                                func.body.as_slice(),
                                &param.name,
                            ) || self.body_forwards_param_in_if_condition(&param.name, func)
                            || self.current_fn_forward_ref_if_params.contains(&param.name)
                            {
                                ownership_mode = OwnershipMode::Owned;
                            }

                            if !self.is_type_copy(&param.type_)
                                && !matches!(
                                    &param.type_,
                                    Type::Reference(_) | Type::MutableReference(_)
                                )
                                && (self.param_has_forward_ref_keep_owned(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ) || self.current_fn_mixed_forwarder_params.contains(&param.name))
                            {
                                ownership_mode = OwnershipMode::Owned;
                            }

                            if !self.ir_cutover.ownership {
                                if self.inferred_borrowed_params.contains(&param.name) {
                                    ownership_mode = OwnershipMode::Borrowed;
                                } else if self.inferred_mut_borrowed_params.contains(&param.name) {
                                    ownership_mode = OwnershipMode::MutBorrowed;
                                }

                                // Converged registry Owned wins over stale first-pass borrow hints
                                // (e.g. imported Copy Vec3 with only field reads).
                                if registry_ownership == Some(OwnershipMode::Owned) {
                                    ownership_mode = OwnershipMode::Owned;
                                }

                                // Copy aggregates pass by value unless the analyzer kept an active
                                // borrow (field-enum-match). Stale registry `Reference(T)` alone
                                // must not emit `&Vec3` formals (bug_copy_vec3_formal_param_not_ref).
                                if self.is_type_copy(formal_type)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        formal_type,
                                    )
                                    && !self.inferred_borrowed_params.contains(&param.name)
                                {
                                    ownership_mode = OwnershipMode::Owned;
                                }

                                if self.is_collection_key_owned_param(param, func) {
                                    ownership_mode = OwnershipMode::Owned;
                                }

                                if !self.is_type_copy(formal_type)
                                    && self.param_passed_to_owned_self_method_arg(
                                        func.body.as_slice(),
                                        &param.name,
                                        func,
                                    )
                                {
                                    ownership_mode = OwnershipMode::Owned;
                                }

                                // Field mutation on params requires &mut when the body mutates fields.
                                if !self.in_trait_impl
                                    && !analyzed.returned_parameters.contains(&param.name)
                                    && analyzed.mutated_parameters.contains(&param.name)
                                    && self.variable_needs_mut(&param.name)
                                    && (!param.is_mutable
                                        || analyzed
                                            .field_mutated_parameters
                                            .contains(&param.name))
                                {
                                    ownership_mode = OwnershipMode::MutBorrowed;
                                }
                            }

                            if self.param_moves_via_struct_literal_init(
                                func.body.as_slice(),
                                &param.name,
                            ) {
                                if matches!(&param.type_, Type::Vec(_)) {
                                    ownership_mode = OwnershipMode::Borrowed;
                                } else {
                                    ownership_mode = OwnershipMode::Owned;
                                }
                            }

                            // Pure delegation wrappers keep &T formals when the body only
                            // forwards to borrowing callees (wdb LsmEngine::get), even under IR cutover.
                            if !self.is_type_copy(&param.type_)
                                && !analyzed.field_extract_parameters.contains(&param.name)
                                && !self.param_moves_via_struct_literal_init(
                                    func.body.as_slice(),
                                    &param.name,
                                )
                                && !matches!(
                                    &param.type_,
                                    Type::Reference(_) | Type::MutableReference(_)
                                )
                                && !self.func_is_pure_forwarding_delegate(func)
                                && !self.param_passed_from_multiple_statements(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.param_has_forward_ref_keep_owned(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.current_fn_mixed_forwarder_params.contains(&param.name)
                                && !self.param_passes_to_wj_owned_sibling_call(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.current_struct_name.as_ref().is_some_and(|sn| {
                                    self.struct_is_owned_engine_key_facade(sn, param)
                                })
                                && !self.is_collection_key_owned_param(param, func)
                                && !self.param_is_single_arg_call_only_delegate(param, func)
                                && (self.inferred_borrowed_params.contains(&param.name)
                                    || (converged_analyzer_borrow
                                        && (crate::codegen::rust::types::is_windjammer_text_type(
                                            &param.type_,
                                        )
                                            || self.is_type_copy(&param.type_)
                                            || self.param_should_emit_borrowed_delegation_formal(
                                                param, func,
                                            )))
                                    || self.param_passed_to_borrowing_callee(
                                        func.body.as_slice(),
                                        &param.name,
                                        func,
                                    ))
                            {
                                ownership_mode = OwnershipMode::Borrowed;
                            } else if !self.is_type_copy(&param.type_)
                                && !matches!(
                                    &param.type_,
                                    Type::Reference(_) | Type::MutableReference(_)
                                )
                                && !self.param_has_forward_ref_keep_owned(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.current_fn_mixed_forwarder_params.contains(&param.name)
                                && !self.param_passes_to_wj_owned_sibling_call(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && self.inferred_mut_borrowed_params.contains(&param.name)
                            {
                                ownership_mode = OwnershipMode::MutBorrowed;
                            }

                            // E0053 FIX: Trait impl parameters MUST match the trait
                            // definition's parameter types exactly. Look up the trait's
                            // method signature and use its ownership for each parameter.
                            if trait_impl_owned_string {
                                ownership_mode = OwnershipMode::Owned;
                            } else if self.in_trait_impl {
                                if let Some(trait_own) = self
                                    .current_trait_impl_name
                                    .as_ref()
                                    .and_then(|trait_name| {
                                        let methods = self
                                            .analyzed_trait_methods
                                            .get(trait_name.as_str())
                                            .or_else(|| {
                                                trait_name
                                                    .rfind("::")
                                                    .map(|i| &trait_name[i + 2..])
                                                    .and_then(|key| {
                                                        self.analyzed_trait_methods.get(key)
                                                    })
                                            });
                                        methods.and_then(|m| {
                                            m.get(func.name.as_str()).and_then(|trait_fn| {
                                                // Use position-based lookup: impl param names
                                                // may differ from trait (e.g., ctx vs app).
                                                trait_fn.decl.parameters.get(param_idx)
                                                    .and_then(|trait_param| {
                                                        trait_fn.inferred_ownership.get(&trait_param.name).copied()
                                                    })
                                                    .or_else(|| {
                                                        trait_fn.inferred_ownership.get(&param.name).copied()
                                                    })
                                            })
                                        })
                                    })
                                {
                                    ownership_mode = trait_own;
                                }
                            }

                            if ownership_mode != OwnershipMode::Owned
                                && self.param_only_used_in_discarding_let_binding(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                            {
                                ownership_mode = OwnershipMode::Owned;
                            }

                            let registry_param_ty = self
                                .get_signature_with_global(&func.name)
                                .and_then(|s| s.param_types.get(param_idx).cloned())
                                .or_else(|| analyzed.inferred_param_types.get(param_idx).cloned());
                            let copy_aggregate_ref_formal = if self.in_trait_impl {
                                None
                            } else {
                                registry_param_ty.as_ref().and_then(|ty| {
                                    if let Type::Reference(inner) = ty {
                                        if ownership_mode == OwnershipMode::Borrowed
                                            && self.inferred_borrowed_params.contains(&param.name)
                                            && self.is_type_copy(formal_type)
                                            && !crate::type_classification::is_copy_pass_by_value_formal(
                                                formal_type,
                                            )
                                        {
                                            Some(format!("&{}", self.type_to_rust(inner)))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                            };

                            copy_aggregate_ref_formal.unwrap_or_else(|| match ownership_mode {
                                OwnershipMode::Owned => self.type_to_rust(formal_type),
                                OwnershipMode::MutBorrowed if self.is_type_copy(formal_type) => {
                                    format!("&mut {}", self.type_to_rust(formal_type))
                                }
                                OwnershipMode::MutBorrowed => {
                                    format!("&mut {}", self.type_to_rust(formal_type))
                                }
                                OwnershipMode::Borrowed if self.is_type_copy(formal_type) => {
                                    if self.inferred_borrowed_params.contains(&param.name) {
                                        format!("&{}", self.type_to_rust(formal_type))
                                    } else {
                                        self.type_to_rust(formal_type)
                                    }
                                }
                                OwnershipMode::Borrowed => {
                                    let is_string = matches!(formal_type, Type::String)
                                        || matches!(formal_type, Type::Custom(ref name) if name == "string");
                                    if is_string && !trait_impl_owned_string {
                                        let registry_str_ref = self
                                            .get_signature_with_global(&func.name)
                                            .and_then(|sig| sig.param_types.get(param_idx))
                                            .is_some_and(|ty| {
                                                matches!(
                                                    ty,
                                                    Type::Reference(inner)
                                                        if matches!(
                                                            &**inner,
                                                            Type::Custom(n) if n == "str"
                                                        )
                                                )
                                            });
                                        if self.str_ref_optimized_params.contains(&param.name)
                                            || registry_str_ref
                                        {
                                            if self.param_only_forwarded_to_qualified_collection_key_callee(
                                                func.body.as_slice(),
                                                &param.name,
                                                func,
                                            ) && func.parent_type.is_none() {
                                                self.type_to_rust(formal_type)
                                            } else {
                                                "&str".to_string()
                                            }
                                        } else {
                                            "&String".to_string()
                                        }
                                    } else {
                                        format!("&{}", self.type_to_rust(formal_type))
                                    }
                                }
                            })
                        }
                    }
                };

                // WINDJAMMER LIFETIME INFERENCE: Add 'a lifetime to reference parameters
                // when the function needs explicit lifetime annotations.
                type_str = if needs_lifetime && param.name != "self" {
                    if let Some(stripped) = type_str.strip_prefix("&mut ") {
                        format!("&'a mut {}", stripped)
                    } else if let Some(stripped) = type_str.strip_prefix("&") {
                        format!("&'a {}", stripped)
                    } else {
                        type_str
                    }
                } else {
                    type_str
                };

                // Copy aggregates: strip spurious readonly `&T` from field reads; keep `&mut T`
                // for direct mutation and passthrough to mutating callees.
                if param.name != "self"
                    && self.is_type_copy(&param.type_)
                    && !crate::type_classification::is_copy_pass_by_value_formal(&param.type_)
                    && !crate::analyzer::field_enum_borrow::param_only_used_as_field_enum_match_scrutinee(
                        &param.name,
                        func.body.as_slice(),
                    )
                    && type_str.starts_with('&')
                    && !type_str.starts_with("&mut ")
                {
                    type_str = self.type_to_rust(&param.type_);
                }

                // Explicitly Copy types kept as owned (instead of &mut) still need `mut`
                // when the analyzer inferred MutBorrowed — the body mutates the value.
                let copy_mut_as_owned = self.is_type_copy(formal_type)
                    && !type_str.starts_with('&')
                    && self.inferred_mut_borrowed_params.contains(&param.name);

                // Copy scalar owned formals pass by value — clear stale borrow metadata.
                if self.is_type_copy(formal_type)
                    && !type_str.starts_with('&')
                    && crate::type_classification::is_copy_pass_by_value_formal(formal_type)
                {
                    self.inferred_borrowed_params.remove(&param.name);
                    self.inferred_mut_borrowed_params.remove(&param.name);
                }

                // Keep call-site borrow tracking aligned with emitted Rust formals.
                // `name: &String` must not get `contains(&name)` at call sites.
                if param.name != "self" {
                    if type_str.starts_with("&mut ") || type_str.starts_with("&'a mut ") {
                        self.emitted_rust_ref_formals.insert(param.name.clone());
                        self.inferred_mut_borrowed_params.insert(param.name.clone());
                        self.inferred_borrowed_params.remove(&param.name);
                        if param.name != "self" {
                            let user_arg_idx = func
                                .parameters
                                .iter()
                                .filter(|p| p.name != "self")
                                .position(|p| p.name == param.name)
                                .unwrap_or(param_idx);
                            self.current_fn_emitted_mut_arg_indices
                                .insert(user_arg_idx);
                        }
                    } else if type_str.starts_with('&') {
                        self.emitted_rust_ref_formals.insert(param.name.clone());
                        self.inferred_borrowed_params.insert(param.name.clone());
                        self.inferred_mut_borrowed_params.remove(&param.name);
                        if type_str == "&str" || type_str.starts_with("&'a str") {
                            self.str_ref_optimized_params.insert(param.name.clone());
                        }
                    } else {
                        self.emitted_rust_ref_formals.remove(&param.name);
                        // Owned emitted formal: stale analyzer borrow metadata must not
                        // suppress call-site `&` (wdb put_value → key_in_latest_base(&key)).
                        self.inferred_borrowed_params.remove(&param.name);
                        self.inferred_mut_borrowed_params.remove(&param.name);
                        self.str_ref_optimized_params.remove(&param.name);
                    }
                }

                // TDD FIX: Auto-infer `mut` for owned parameters
                // THE WINDJAMMER WAY: Users don't track mutability - the compiler does.
                // If a parameter has mutating method calls or field mutations,
                // the binding needs `mut` even if not explicitly written.
                let auto_needs_mut = param.name != "self"
                    && !param.is_mutable
                    && matches!(type_str.as_str(), s if !s.starts_with("&"))
                    && (self.variable_needs_mut(&param.name) || copy_mut_as_owned);
                let mut_prefix = if (param.is_mutable || auto_needs_mut)
                    && !type_str.starts_with('&')
                {
                    "mut "
                } else {
                    ""
                };

                // TDD FIX: Prefix unused parameter names with `_` to suppress warnings
                let display_name = if unused_params.contains(&param.name) {
                    format!("_{}", param.name)
                } else {
                    param.name.clone()
                };

                // Check if this is a pattern parameter
                if let Some(pattern) = &param.pattern {
                    // Generate pattern: type syntax
                    format!(
                        "{}{}: {}",
                        mut_prefix,
                        self.generate_pattern(pattern),
                        type_str
                    )
                } else {
                    // Simple name: type syntax
                    format!("{}{}: {}", mut_prefix, display_name, type_str)
                }
            })
            .collect()
    }
}
