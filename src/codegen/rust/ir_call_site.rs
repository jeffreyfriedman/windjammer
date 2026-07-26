//! IR-driven call-site argument coercion for Rust codegen.
//!
//! When `IrCutoverConfig.call_sites` is enabled, applies `encode_call_argument`
//! using callee signature expectations instead of heuristic borrow passes.

use crate::analyzer::SignatureRegistry;
use crate::codegen::rust::generator::CodeGenerator;
use crate::ir::coercion::compute_coercion;
use crate::ir::coercion::CoercionKind;
use crate::ir::signature_bridge::{safety_type_from_parser_type, safety_type_from_signature_param};
use crate::ir::safety_type::{BaseType, OwnedType, Region, SafetyType};
use crate::ir::target_encodings::{apply_coercion, Target};
use crate::parser::{Expression, Literal, Statement, Type};

impl<'ast> CodeGenerator<'ast> {
    /// Apply IR-driven coercion to a call-site argument when call_sites cutover is on.
    pub(crate) fn apply_ir_call_site_coercion(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        local_sig: Option<&crate::analyzer::FunctionSignature>,
        skip_on_ownership_collision: bool,
        receiver_type_name: Option<&str>,
        user_arg_count: Option<usize>,
    ) -> Option<String> {
        if !self.ir_cutover.call_sites {
            return None;
        }

        let simple_callee = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let local_emitted_mut = self
            .function_emitted_mut_arg_indices
            .get(callee_name)
            .or_else(|| self.function_emitted_mut_arg_indices.get(simple_callee))
            .is_some_and(|indices| indices.contains(&arg_index));
        let has_preregistered_local_callee = [callee_name, simple_callee].iter().any(|name| {
            registry
                .get_signature(name)
                .is_some_and(|sig| sig.emitted_rust_ref_params.is_some())
        });
        if skip_on_ownership_collision
            && crate::codegen::rust::call_signature_resolution::has_ownership_collision_for_call(
                self, callee_name,
            )
            && !local_emitted_mut
            && !has_preregistered_local_callee
        {
            return None;
        }

        // User-written `&x` / `&mut x` at the call site: preserve generated Rust as-is.
        if crate::codegen::rust::expression_helpers::is_reference_expression(arg_expr) {
            return Some(arg_str.to_string());
        }

        // Never skip auto-clone when the callee emits owned formals: analyzer/global
        // stubs may still say Borrowed (`&Vec`) while codegen emits `Vec` (WDB-056/059).
        let emits_owned_formal = self.ir_callee_arg_emits_owned_contract(
            registry,
            callee_name,
            arg_index,
            user_arg_count,
            local_sig,
        );
        let skip_auto_clone_for_borrow = !emits_owned_formal
            && (self.ir_callee_arg_expects_shared_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                local_sig,
            ) || self.ir_callee_arg_expects_mut_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                local_sig,
            ));
        let skip_auto_clone_for_field_extract = matches!(arg_expr, Expression::Identifier { .. }
            if self.callee_param_field_extracts_by_name(callee_name, arg_index));
        let collecting_ref_vec = matches!(arg_expr, Expression::Identifier { name, .. }
            if self.borrowed_iterator_vars.contains(name)
                && self.current_function_return_type.as_ref().is_some_and(|rt| {
                    matches!(rt, Type::Vec(inner) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_)))
                }));
        let mut prepared_arg = match arg_expr {
            Expression::Identifier { name, .. }
                if !skip_auto_clone_for_borrow
                    && !skip_auto_clone_for_field_extract
                    && !collecting_ref_vec =>
            {
                self.maybe_auto_clone(name, arg_str)
            }
            // Field paths (`record.key`) moved into owned formals + reused in loops
            // need `.clone()`; identifier-only auto-clone misses them (WDB-059).
            Expression::FieldAccess { .. } | Expression::Index { .. }
                if !skip_auto_clone_for_borrow && !skip_auto_clone_for_field_extract =>
            {
                self.maybe_auto_clone_expr_path(arg_expr, arg_str)
            }
            _ => arg_str.to_string(),
        };

        let method_simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && arg_index == 0
            && crate::analyzer::stdlib_method_traits::is_storage_method(method_simple)
            && !prepared_arg.ends_with(".to_string()")
        {
            return Some(format!(
                "{}.to_string()",
                prepared_arg.trim_start_matches('&')
            ));
        }

        let mut sig = if receiver_type_name.is_none() && !callee_name.contains("::") {
            registry
                .get_signature(callee_name)
                .cloned()
                .or_else(|| local_sig.cloned())
        } else {
            local_sig
                .cloned()
                .or_else(|| registry.get_signature(callee_name).cloned())
        }
        .or_else(|| registry.lookup_method(callee_name).cloned());

        if let Some((receiver_ty, method)) = callee_name.rsplit_once("::") {
            let arg_count = user_arg_count.unwrap_or(arg_index + 1);
            if let Some(method_sig) =
                self.resolve_method_function_signature(receiver_ty, method, arg_count)
            {
                let prefer_method = sig.as_ref().is_none_or(|local| {
                    let local_idx = local.arg_param_index(arg_index);
                    let method_idx = method_sig.arg_param_index(arg_index);
                    let local_ref = local
                        .param_types
                        .get(local_idx)
                        .is_some_and(|t| matches!(t, Type::Reference(_)));
                    let method_ref = method_sig
                        .param_types
                        .get(method_idx)
                        .is_some_and(|t| matches!(t, Type::Reference(_)));
                    let local_owned = matches!(
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                            local, local_idx,
                        ),
                        crate::analyzer::OwnershipMode::Owned
                    );
                    (method_ref && !local_ref) || (local_owned && method_ref)
                }) || crate::codegen::rust::signature_promotion::method_registry_reflects_emitted_owned(
                    &method_sig,
                ) || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &method_sig,
                    method_sig.arg_param_index(arg_index),
                );
                if prefer_method {
                    sig = Some(method_sig);
                }
            }
        }

        if let Some(global) = self.global_signature_registry.as_ref() {
            if let Some(global_sig) = global.get_signature(callee_name) {
                let global_idx = global_sig.arg_param_index(arg_index);
                let method_registry_owned = callee_name.rsplit_once("::").is_some_and(
                    |(receiver_ty, method)| {
                        let arg_count = user_arg_count.unwrap_or(arg_index + 1);
                        self.resolve_method_function_signature(receiver_ty, method, arg_count)
                            .map(|method_sig| {
                                crate::codegen::rust::signature_promotion::method_registry_reflects_emitted_owned(
                                    &method_sig,
                                ) || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    &method_sig,
                                    method_sig.arg_param_index(arg_index),
                                )
                            })
                            .unwrap_or(false)
                    },
                );
                let prefer_global = !method_registry_owned
                    && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        global_sig,
                        global_idx,
                    )
                    && sig.as_ref().is_none_or(|local_sig| {
                    let idx = local_sig.arg_param_index(arg_index);
                    let local_eff =
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                            local_sig, idx,
                        );
                    let global_idx = global_sig.arg_param_index(arg_index);
                    let global_eff =
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                            global_sig, global_idx,
                        );
                    let global_ref_ty = global_sig
                        .param_types
                        .get(global_idx)
                        .is_some_and(|t| matches!(t, Type::Reference(_)));
                    let local_owned = matches!(local_eff, crate::analyzer::OwnershipMode::Owned);
                    let global_borrows = matches!(
                        global_eff,
                        crate::analyzer::OwnershipMode::Borrowed
                            | crate::analyzer::OwnershipMode::MutBorrowed
                    ) || global_ref_ty
                        || matches!(
                            global_sig.param_ownership.get(global_idx),
                            Some(crate::analyzer::OwnershipMode::Borrowed)
                                | Some(crate::analyzer::OwnershipMode::MutBorrowed)
                        );
                    local_owned && global_borrows
                });
                if prefer_global {
                    sig = Some(global_sig.clone());
                }
            }
        }

        if callee_name.starts_with("Self::") {
            if let Some(ref tn) = self.current_struct_name {
                if let Some(method) = callee_name.strip_prefix("Self::") {
                    if let Some(ms) = self.lookup_method_signature(tn, method) {
                        sig = Some(ms.to_function_signature());
                    }
                }
            }
        }

        let Some(sig) = sig
        else {
            let mut finished = self.finish_runtime_std_call_arg(
                callee_name,
                arg_index,
                arg_expr,
                prepared_arg,
                None,
                receiver_type_name,
            );
            self.apply_registry_borrow_to_call_arg(
                &mut finished,
                arg_expr,
                receiver_type_name,
                method_simple,
                arg_index,
                user_arg_count,
            );
            return Some(finished);
        };

        // Inline `mod gpu { … }` callees: don't coerce string literals to owned String
        // when module origin can't be verified (single-file conservative guard).
        if self.inline_module_qualified_call(callee_name)
            && matches!(
                arg_expr,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            )
        {
            return Some(prepared_arg);
        }

        let receiver_is_map = receiver_type_name
            .is_some_and(crate::codegen::rust::stdlib_method_traits::is_map_type_name);
        let receiver_is_set = receiver_type_name
            .is_some_and(crate::codegen::rust::stdlib_method_traits::is_set_type_name);

        let param_idx = sig.arg_param_index(arg_index);
        let mut expected = safety_type_from_signature_param(&sig, param_idx);
        let callee_module = crate::codegen::rust::stdlib_method_traits::resolve_runtime_std_module(
            callee_name.split("::").next().unwrap_or(""),
            receiver_type_name,
        );
        let inferred_arg_type = self.infer_expression_type(arg_expr);
        let runtime_param_type = sig
            .formal_param_type(param_idx)
            .or(inferred_arg_type.as_ref());
        if let Some(formal_ty) = runtime_param_type {
            if crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow(
                Some(&sig),
                arg_index,
            ) {
                expected.ownership = OwnedType::Ref(Region::fresh(7));
            } else if crate::codegen::rust::string_utilities::param_is_rust_str_ref(formal_ty) {
                expected.base = BaseType::String;
                expected.ownership = OwnedType::Ref(Region::fresh(5));
            } else if (crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
                callee_module,
            ) || crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow(
                Some(&sig),
                arg_index,
            ))
                && crate::codegen::rust::types::is_windjammer_text_type(formal_ty)
            {
                expected.base = BaseType::String;
                expected.ownership = OwnedType::Ref(Region::fresh(8));
            }
        }
        if sig.param_types.get(param_idx).is_some_and(|t| {
            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
        }) && (crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
            &sig, param_idx,
        ) || !crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
            &sig, param_idx,
        ))
        {
            expected.base = BaseType::String;
            expected.ownership = OwnedType::Ref(Region::fresh(5));
        } else if let Some(ownership) = sig.param_ownership.get(param_idx).copied() {
            use crate::analyzer::OwnershipMode;
            if matches!(
                ownership,
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
            ) {
                if callee_module == "json" {
                    expected.ownership = OwnedType::Ref(Region::fresh(7));
                } else if crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
                    callee_module,
                ) {
                    expected.base = BaseType::String;
                    expected.ownership = OwnedType::Ref(Region::fresh(8));
                }
            }
        }
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && crate::codegen::rust::stdlib_method_traits::is_string_pattern_method(method_simple)
        {
            expected.base = BaseType::String;
            expected.ownership = OwnedType::Ref(Region::fresh(9));
        } else if crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
            &sig,
            arg_index,
            receiver_type_name,
        ) {
            expected.ownership = OwnedType::Ref(Region::fresh(4));
        } else if arg_index == 0
            && crate::analyzer::stdlib_method_traits::is_set_lookup_method(method_simple)
            && receiver_is_set
        {
            let arg_already_borrowed = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.identifier_already_ref(name)
                        || self.inferred_borrowed_params.contains(name)
                        || self.emitted_rust_ref_formals.contains(name)
            );
            if !arg_already_borrowed {
                expected.ownership = OwnedType::Ref(Region::fresh(6));
            }
        } else if arg_index == 0
            && crate::analyzer::stdlib_method_traits::is_storage_method(method_simple)
            && (receiver_is_map || receiver_is_set)
            && matches!(expected.base, BaseType::String | BaseType::Custom(_))
            && !crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
                callee_module,
            )
        {
            expected.ownership = OwnedType::Owned;
        }
        let actual = self.infer_actual_safety_type(arg_expr, prepared_arg.as_str());
        let mut kind = compute_coercion(&actual, &expected);
        if crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
            && (crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                &sig,
                arg_index,
            ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig,
                param_idx,
            ))
            && matches!(kind, CoercionKind::Clone | CoercionKind::ToOwnedString)
        {
            kind = CoercionKind::Identity;
        }
        if matches!(kind, CoercionKind::Clone)
            && self.callee_param_field_extracts_by_name(callee_name, arg_index)
        {
            kind = CoercionKind::Identity;
        }
        if matches!(kind, CoercionKind::Clone) {
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.in_user_written_closure && self.user_closure_params.contains(name) {
                    kind = CoercionKind::Identity;
                }
            }
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.identifier_already_ref(name)
                && matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
            {
                kind = CoercionKind::Identity;
            }
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.borrowed_iterator_vars.contains(name)
                && matches!(kind, CoercionKind::Clone)
            {
                kind = CoercionKind::Identity;
            }
            if matches!(kind, CoercionKind::Clone) {
                let user_param = |idx: usize| {
                    if sig.has_self_receiver_slot() {
                        idx > 0
                    } else {
                        true
                    }
                };
                let this_arg_expects_borrow =
                    self.ir_sig_arg_expects_shared_borrow(&sig, arg_index);
                let arg_is_current_fn_param = self
                    .current_function_params
                    .iter()
                    .any(|p| p.name == *name);
                if this_arg_expects_borrow && arg_is_current_fn_param {
                    kind = CoercionKind::Identity;
                }
            }
        }
        // `.clone()` already produces an owned value — never prefix shared `&`.
        // MutBorrow keeps going: clone is stripped just below before apply_coercion.
        if prepared_arg.ends_with(".clone()")
            && matches!(kind, CoercionKind::Borrow)
            && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
        {
            kind = CoercionKind::Identity;
        }
        if matches!(kind, CoercionKind::Clone) {
            let formal_ty = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx));
            if formal_ty.is_some_and(|t| self.is_type_copy(t)) {
                // Owned Copy formal: `&T` auto-copies at the call site — no `.clone()`.
                kind = CoercionKind::Identity;
            } else if let Some(ty) = self.infer_expression_type(arg_expr) {
                let pointee = match &ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                if self.is_type_copy(pointee) {
                    kind = CoercionKind::Identity;
                }
            }
        }
        // Explicit `*binding` on Copy types: Rust auto-borrows at the call site — no `&*`.
        if matches!(
            arg_expr,
            Expression::Unary {
                op: crate::parser::UnaryOp::Deref,
                ..
            }
        ) && matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
        {
            let callee_formal_is_copy = sig
                .formal_param_type(param_idx)
                .is_some_and(|t| self.is_type_copy(t));
            if callee_formal_is_copy {
                kind = CoercionKind::Identity;
            } else if let Expression::Unary { operand, .. } = arg_expr {
                if self.infer_expression_type(operand).as_ref().is_some_and(|t| {
                    matches!(t, Type::Reference(inner) | Type::MutableReference(inner)
                        if self.is_type_copy(inner.as_ref()))
                }) {
                    kind = CoercionKind::Identity;
                }
            }
        }
        // Borrow coercion on `binding.clone()` → use `&binding` (clone-before-borrow is redundant).
        if prepared_arg.ends_with(".clone()")
            && matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
            && matches!(arg_expr, Expression::Identifier { .. })
        {
            prepared_arg = prepared_arg
                .trim_end_matches(".clone()")
                .to_string();
        }
        let resolved_kind = kind;
        let coerced = apply_coercion(&resolved_kind, prepared_arg.as_str(), Target::Rust);
        let mut coerced = self.finalize_ir_call_arg(arg_expr, prepared_arg.as_str(), &coerced);

        // Unified borrow guard: registry Reference(T) / Borrowed ownership must reach call sites
        // even when IR coercion chose Identity (stale effective ownership metadata).
        let arg_already_rust_ref = matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.identifier_already_ref(name) || self.identifier_already_mut_ref(name)
        );
        let formal_is_copy = sig
            .formal_param_type(param_idx)
            .is_some_and(|t| self.is_type_copy(t));
        let mut borrow_decision =
            crate::codegen::rust::call_site_borrow::should_borrow_at_call_site_with_copy_check(
                &sig,
                arg_index,
                arg_expr,
                &coerced,
                method_simple,
                arg_already_rust_ref,
                receiver_type_name,
                formal_is_copy,
            );
        if matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.identifier_already_ref(name) || self.identifier_already_mut_ref(name)
        ) {
            borrow_decision.add_ref = false;
            borrow_decision.add_mut_ref = false;
        }
        // Stale multipass metadata may infer borrow for plain `string` formals on user
        // free functions that actually emit owned `String` (circular-dep convergence).
        if receiver_type_name.is_none()
            && crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn(
                &self.signature_registry,
                callee_name,
                &sig,
                param_idx,
                arg_index,
            )
        {
            borrow_decision.add_ref = false;
            borrow_decision.add_mut_ref = false;
        }
        crate::codegen::rust::call_site_borrow::apply_call_site_borrow(
            &borrow_decision,
            &mut coerced,
        );
        if crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn(
            &self.signature_registry,
            callee_name,
            &sig,
            param_idx,
            arg_index,
        ) && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            coerced = coerced[1..].to_string();
        }
        if self.ir_callee_arg_expects_mut_borrow(
            registry,
            callee_name,
            arg_index,
            user_arg_count,
            Some(&sig),
        )
            && matches!(arg_expr, Expression::Identifier { .. })
            && !coerced.starts_with("&mut ")
        {
            if let Expression::Identifier { name, .. } = arg_expr {
                if !self.identifier_already_mut_ref(name) {
                    if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                        coerced = format!(
                            "&mut {}",
                            crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                        );
                    } else {
                        coerced = format!("&mut {coerced}");
                    }
                }
            }
        }

        // Method-registry / global converged signatures must win over stale call-site
        // metadata (wdb `engine.put` delegation, forward refs to later impl methods).
        self.apply_registry_borrow_to_call_arg(
            &mut coerced,
            arg_expr,
            receiver_type_name,
            method_simple,
            arg_index,
            user_arg_count,
        );

        // Storage methods (HashMap.insert, etc.) need owned String keys from literals.
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && arg_index == 0
            && crate::analyzer::stdlib_method_traits::is_storage_method(method_simple)
            && !crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
                callee_module,
            )
            && !coerced.ends_with(".to_string()")
        {
            return Some(format!(
                "{}.to_string()",
                coerced.trim_start_matches('&')
            ));
        }
        let mut coerced = self.finish_runtime_std_call_arg(
            callee_name,
            arg_index,
            arg_expr,
            coerced,
            Some(&sig),
            receiver_type_name,
        );
        if coerced.ends_with(".clone()") {
            let this_arg_expects_borrow =
                self.ir_sig_arg_expects_shared_borrow(&sig, arg_index);
            match arg_expr {
                Expression::Identifier { name, .. } => {
                    let arg_is_fn_param = self
                        .current_function_params
                        .iter()
                        .any(|p| p.name == *name);
                    if self.borrowed_iterator_vars.contains(name)
                        && self.current_function_return_type.as_ref().is_some_and(|rt| {
                            matches!(rt, Type::Vec(inner) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_)))
                        })
                    {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut coerced,
                        );
                    } else if arg_is_fn_param && this_arg_expects_borrow {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut coerced,
                        );
                    }
                }
                Expression::FieldAccess { object, .. } => {
                    if this_arg_expects_borrow
                        && matches!(
                            &**object,
                            Expression::Identifier { name, .. } if name == "self"
                        )
                    {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut coerced,
                        );
                    }
                }
                _ => {}
            }
        }
        let pidx = sig.arg_param_index(arg_index);
        let simple_callee = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let callee_sig = callee_name
            .rsplit_once("::")
            .and_then(|(rt, method)| {
                self.resolve_method_function_signature(
                    rt,
                    method,
                    user_arg_count.unwrap_or(arg_index + 1),
                )
            })
            .or_else(|| {
                self.signature_registry
                    .get_signature(callee_name)
                    .cloned()
                    .or_else(|| self.signature_registry.get_signature(simple_callee).cloned())
            })
            .unwrap_or(sig.clone());
        let callee_pidx = callee_sig.arg_param_index(arg_index);
        let runtime_std_needs_borrow = {
            let inferred = self.infer_expression_type(arg_expr);
            crate::codegen::rust::stdlib_method_traits::runtime_std_call_arg_needs_auto_borrow(
                callee_module,
                simple_callee,
                Some(&callee_sig),
                arg_index,
                inferred.as_ref(),
                arg_expr,
                receiver_type_name,
            )
        };
        if !matches!(
            resolved_kind,
            CoercionKind::Identity | CoercionKind::Borrow | CoercionKind::MutBorrow
        )
            && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(&callee_sig, callee_pidx)
            && !runtime_std_needs_borrow
            && !(crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                && crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
                    callee_module,
                ))
        {
            let callee_borrows_text = self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                || callee_sig.param_types.get(callee_pidx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                });
            if !callee_borrows_text {
            let skip_clone_for_user_closure_param = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.in_user_written_closure && self.user_closure_params.contains(name)
            );
            if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                let param_is_ref = callee_sig
                    .param_types
                    .get(callee_pidx)
                    .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
                if !param_is_ref {
                    coerced = coerced[1..].to_string();
                }
            }
            if skip_clone_for_user_closure_param {
                // User closure params pass through unchanged (e.g. |e| predicate(e)).
            } else if matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.identifier_already_ref(name)
                        || self.emitted_rust_ref_formals.contains(name)
            ) && !coerced.ends_with(".clone()")
            {
                let formal_is_copy = callee_sig
                    .formal_param_type(callee_pidx)
                    .is_some_and(|t| self.is_type_copy(t));
                let callee_borrows_text = self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                    || callee_sig.param_types.get(callee_pidx).is_some_and(|t| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    });
                if !formal_is_copy && !callee_borrows_text {
                    coerced = format!("{}.clone()", coerced.trim_start_matches('&'));
                }
            } else if !coerced.ends_with(".clone()") {
                let skip_iter_ref_collect = matches!(arg_expr, Expression::Identifier { name, .. }
                    if self.borrowed_iterator_vars.contains(name)
                        && self.current_function_return_type.as_ref().is_some_and(|rt| {
                            matches!(rt, Type::Vec(inner) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_)))
                        }));
                if !skip_iter_ref_collect {
                let callee_borrows_text = self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                    || callee_sig.param_types.get(callee_pidx).is_some_and(|t| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    });
                if !callee_borrows_text {
                    let actual = self.infer_actual_safety_type(arg_expr, coerced.as_str());
                    let expected = safety_type_from_signature_param(&callee_sig, callee_pidx);
                    if matches!(
                        compute_coercion(&actual, &expected),
                        CoercionKind::Clone
                    ) {
                        coerced = format!("{}.clone()", coerced.trim_start_matches('&'));
                    }
                }
                }
            }
            }
        }
        let _ = pidx;
        if coerced.ends_with(".clone()") {
            let formal_ty = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx));
            if formal_ty.is_some_and(|t| self.is_type_copy(t)) {
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
            } else if let Expression::Identifier { name, .. } = arg_expr {
                if self.local_var_types.get(name).is_some_and(|t| {
                    matches!(
                        t,
                        Type::Reference(inner) | Type::MutableReference(inner)
                            if self.is_type_copy(inner.as_ref())
                    )
                }) {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
                }
            }
        }
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && !crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
            callee_module,
        )
            && sig
            .param_type_for_arg(arg_index)
            .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_string_ref)
        {
            let base = coerced.trim_start_matches('&');
            let base = if base.ends_with(".to_string()") {
                base.to_string()
            } else {
                format!("{}.to_string()", base)
            };
            return Some(format!("&{}", base));
        }
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && !crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
            callee_module,
        )
            && !self.inline_module_qualified_call(callee_name)
            && !crate::codegen::rust::call_signature_resolution::is_lowercase_user_module_qualified_call(
                callee_name,
            )
            && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
            && sig.formal_param_type(param_idx).is_some_and(|t| {
                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    && crate::codegen::rust::types::is_windjammer_text_type(t)
            })
            && matches!(
                sig.param_ownership.get(param_idx),
                Some(crate::analyzer::OwnershipMode::Owned)
            )
            && !coerced.ends_with(".to_string()")
        {
            return Some(format!(
                "{}.to_string()",
                coerced.trim_start_matches('&')
            ));
        }
        let callee_has_ownership_collision =
            crate::codegen::rust::call_signature_resolution::has_ownership_collision_for_call(
                self,
                callee_name,
            );
        let collision_blocks_autoborrow = callee_has_ownership_collision
            && crate::codegen::rust::call_signature_resolution::ownership_collision_blocks_autoborrow(
                callee_name,
            );
        if !collision_blocks_autoborrow {
        crate::codegen::rust::string_utilities::finalize_borrowed_text_call_site_arg(
            Some(&sig),
            arg_index,
            receiver_type_name,
            arg_expr,
            &mut coerced,
            arg_already_rust_ref,
        );
        }
        // Forward-ref: owned caller binding → callee `&T` when registry encodes borrow.
        if let Expression::Identifier { name, .. } = arg_expr {
            let fresh_sig = callee_name
                .rsplit_once("::")
                .and_then(|(rt, method)| {
                    let rt = if rt == "Self" {
                        self.current_struct_name
                            .clone()
                            .unwrap_or_else(|| rt.to_string())
                    } else {
                        rt.to_string()
                    };
                    self.resolve_method_function_signature(
                        rt.as_str(),
                        method,
                        user_arg_count.unwrap_or(arg_index + 1),
                    )
                });
            let borrow_sig = fresh_sig.as_ref().unwrap_or(&sig);
            let borrow_idx = borrow_sig.arg_param_index(arg_index);
            let body_refs: Vec<&Statement<'ast>> =
                self.current_function_body.iter().copied().collect();
            let binding_is_text_param = self.current_function_params.iter().any(|p| {
                p.name == *name && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
            });
            let binding_is_vec_local = self.local_var_types.get(name).is_some_and(|t| {
                matches!(t, Type::Vec(_))
                    || matches!(t, Type::Parameterized(name, _) if name == "Vec")
            });
            let binding_used_as_read_local =
                self.param_used_as_read_operand(body_refs.as_slice(), name);
            let callee_wants_shared = crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                borrow_sig,
                borrow_idx,
            ) || (!crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                borrow_sig,
                borrow_idx,
            ) && borrow_sig.param_types.get(borrow_idx).is_some_and(|t| {
                matches!(t, Type::Reference(_))
                    || crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
            }));
            if !collision_blocks_autoborrow
                && !coerced.starts_with('&')
                && !self.emitted_rust_ref_formals.contains(name)
                && !self.inferred_borrowed_params.contains(name)
                && (binding_is_text_param || binding_used_as_read_local || binding_is_vec_local)
                && callee_wants_shared
            {
                coerced = format!("&{coerced}");
            }
        }
        // Caller-owned text binding → user free-fn callee without confirmed `&str` emission:
        // strip stale multipass `&` (`foo(String)` calling `bar(&x)`). Skip method calls
        // (String::contains etc.) where the library API genuinely expects `&str`.
        if receiver_type_name.is_none()
            && !callee_name.contains("::")
            && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            && !sig.emitted_rust_ref_params.as_ref().is_some_and(|flags| {
                flags.get(param_idx).copied().unwrap_or(false)
            })
            && matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    }) && !self.emitted_rust_ref_formals.contains(name)
            )
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            coerced = coerced[1..].to_string();
        }
        if collision_blocks_autoborrow {
            while coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                coerced = coerced[1..].to_string();
            }
            crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
        }
        // Final IR ownership contract: strip spurious `&` when callee emits owned formals
        // (WDB-056 keys_equal(Vec<u8>, Vec<u8>)), or add borrow when expected.
        self.enforce_call_site_ownership_contract(&mut coerced, arg_expr, &sig, param_idx);
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.caller_owned_non_copy_formal(name)
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                )
            {
                coerced = coerced.trim_start_matches('&').to_string();
            }
        }
        // After borrow stripping / collision clone-stripping: restore `.clone()` when
        // auto-clone analysis says this binding/path is moved and reused (WDB-059).
        coerced = self.ensure_owned_move_clone_for_reuse(
            arg_expr,
            &coerced,
            &sig,
            param_idx,
        );
        Some(coerced)
    }

    /// When a binding/path is moved and reused, and the final argument is passed by
    /// value (no leading `&`), ensure `.clone()` is present (WDB-059).
    fn ensure_owned_move_clone_for_reuse(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        sig: &crate::analyzer::FunctionSignature,
        param_idx: usize,
    ) -> String {
        if arg_str.ends_with(".clone()") || arg_str.starts_with('*') {
            return arg_str.to_string();
        }
        // Shared-ref formals borrow — cloning is unnecessary.
        // Mut-ref formals must keep `&mut binding` — never append `.clone()`.
        if arg_str.starts_with('&') {
            return arg_str.to_string();
        }
        if matches!(
            sig.param_ownership.get(param_idx),
            Some(crate::analyzer::OwnershipMode::MutBorrowed)
                | Some(crate::analyzer::OwnershipMode::Borrowed)
        ) || sig.param_types.get(param_idx).is_some_and(|t| {
            matches!(t, Type::Reference(_) | Type::MutableReference(_))
        }) {
            return arg_str.to_string();
        }
        let Some(ref analysis) = self.auto_clone_analysis else {
            return arg_str.to_string();
        };
        let needs = match arg_expr {
            Expression::Identifier { name, .. } => {
                analysis.needs_clone_anywhere(name)
                    || analysis
                        .needs_clone(name, self.current_statement_idx)
                        .is_some()
            }
            Expression::FieldAccess { .. } | Expression::Index { .. } => {
                Self::auto_clone_expr_path(arg_expr).is_some_and(|path| {
                    analysis.needs_clone_anywhere(&path)
                        || analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                })
            }
            _ => false,
        };
        if needs {
            format!("{arg_str}.clone()")
        } else {
            arg_str.to_string()
        }
    }

    /// Enforce the ownership contract for a single call-site argument by computing
    /// actual vs expected safety types and applying the coercion (strip `&`, add `.clone()`, etc.).
    /// Uses `emitted_owned_arg_contract` to handle stale analyzer metadata.
    pub(crate) fn enforce_call_site_ownership_contract(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        sig: &crate::analyzer::FunctionSignature,
        param_idx: usize,
    ) {
        let expected = safety_type_from_signature_param(sig, param_idx);
        let actual = self.infer_call_arg_actual_safety_type(arg_expr, coerced.as_str());
        let force_owned = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
            sig, param_idx,
        );
        crate::ir::coercion::enforce_ownership_contract_on_coerced_arg_with_force_owned(
            coerced,
            &actual,
            &expected,
            force_owned,
            false,
        );
    }

    pub(crate) fn apply_registry_borrow_to_call_arg(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        receiver_type_name: Option<&str>,
        method: &str,
        arg_index: usize,
        user_arg_count: Option<usize>,
    ) {
        let (Some(_rt), Expression::Identifier { name, .. }) = (receiver_type_name, arg_expr) else {
            return;
        };
        if self.emitted_rust_ref_formals.contains(name) || self.identifier_already_ref(name) {
            return;
        }
        let arg_count = user_arg_count.unwrap_or(arg_index + 1);
        let mut receiver_types: Vec<&str> = Vec::new();
        if let Some(rt) = receiver_type_name {
            receiver_types.push(rt);
        }
        if let Some(sn) = self.current_struct_name.as_deref() {
            if !receiver_types.contains(&sn) {
                receiver_types.push(sn);
            }
        }
        for rt in &receiver_types {
            if let Some(sig) =
                self.resolve_method_function_signature(rt, method, arg_count)
            {
                let pidx = sig.arg_param_index(arg_index);
                let wants_mut = sig.param_types.get(pidx).is_some_and(|t| {
                    matches!(t, Type::MutableReference(_))
                }) || matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        &sig, arg_index,
                    ),
                    crate::analyzer::OwnershipMode::MutBorrowed
                );
                if wants_mut {
                    if coerced.starts_with("&mut ") {
                        return;
                    }
                    if coerced.starts_with('&') {
                        *coerced = format!(
                            "&mut {}",
                            crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                        );
                    } else {
                        *coerced = format!("&mut {coerced}");
                    }
                    return;
                }
            }
        }
        if coerced.starts_with('&') {
            return;
        }
        for rt in &receiver_types {
            if self.method_registry_arg_expects_shared_borrow(rt, method, arg_index, arg_count) {
                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(coerced);
                return;
            }
        }
        if let Some(resolved) = self.resolve_call_signature_with_global(
            method,
            receiver_type_name,
            arg_count,
        ) {
            let pidx = resolved.sig.arg_param_index(arg_index);
            if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                &resolved.sig,
                pidx,
            ) {
                return;
            }
            if resolved.sig.param_types.get(pidx).is_some_and(|t| {
                matches!(t, Type::Reference(_))
            }) {
                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(coerced);
            }
        }
    }

    /// Runtime std modules declare owned WJ params but Rust takes `&T` / `&str`.
    pub(crate) fn finish_runtime_std_call_arg(
        &self,
        callee_name: &str,
        arg_index: usize,
        arg_expr: &Expression<'ast>,
        mut coerced: String,
        signature: Option<&crate::analyzer::FunctionSignature>,
        receiver_type_name: Option<&str>,
    ) -> String {
        if coerced.contains("string_to_ffi(") {
            return coerced;
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.identifier_already_ref(name) {
                return coerced;
            }
        }
        let module = callee_name.split("::").next().unwrap_or("");
        let method = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let inferred_type = self.infer_expression_type(arg_expr);
        let effective_module = crate::codegen::rust::stdlib_method_traits::resolve_runtime_std_module(
            module,
            receiver_type_name,
        );

        if !coerced.starts_with('&')
            && crate::codegen::rust::stdlib_method_traits::runtime_std_call_arg_needs_auto_borrow(
                effective_module,
                method,
                signature,
                arg_index,
                inferred_type.as_ref(),
                arg_expr,
                receiver_type_name,
            )
        {
            coerced = crate::ir::target_encodings::rust_shared_borrow(&coerced);
        }

        let signature_for_borrow = if crate::codegen::rust::call_signature_resolution::is_external_module_qualified_call(
            callee_name,
        ) {
            self.global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(callee_name))
                .or(signature)
        } else {
            signature
        };

        if !coerced.starts_with('&') {
            if let Some(sig) = signature_for_borrow {
                let idx = sig.arg_param_index(arg_index);
                let param_ty = sig
                    .formal_param_type(idx)
                    .or_else(|| sig.param_types.get(idx));
                let effective_ownership =
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, arg_index,
                    );
                let callee_borrows_text = param_ty.is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_string_ref(t)
                        || crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                }) || (matches!(
                    effective_ownership,
                    crate::analyzer::OwnershipMode::Borrowed
                        | crate::analyzer::OwnershipMode::MutBorrowed
                ) && (param_ty
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                    || inferred_type
                        .as_ref()
                        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)));
                if callee_borrows_text
                    && matches!(
                        arg_expr,
                        Expression::Identifier { .. } | Expression::FieldAccess { .. }
                    )
                    && !matches!(
                        arg_expr,
                        Expression::Identifier { name, .. }
                            if self.identifier_already_ref(name)
                    )
                {
                    coerced = crate::ir::target_encodings::rust_shared_borrow(&coerced);
                }
            } else if crate::codegen::rust::call_signature_resolution::is_external_module_qualified_call(
                callee_name,
            ) {
                if let Some(global) = self.global_signature_registry.as_ref() {
                    if let Some(global_sig) = global.get_signature(callee_name) {
                        let effective =
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                global_sig, arg_index,
                            );
                        if matches!(
                            effective,
                            crate::analyzer::OwnershipMode::Borrowed
                                | crate::analyzer::OwnershipMode::MutBorrowed
                        ) && matches!(
                            arg_expr,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        ) && !matches!(
                            arg_expr,
                            Expression::Identifier { name, .. }
                                if self.identifier_already_ref(name)
                        ) {
                            coerced =
                                crate::ir::target_encodings::rust_shared_borrow(&coerced);
                        }
                    }
                }
            }
        }

        if crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
            effective_module,
        )
            && matches!(
                arg_expr,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            )
            && coerced.ends_with(".to_string()")
        {
            coerced = coerced
                .trim_end_matches(".to_string()")
                .to_string();
        }

        coerced
    }

    /// Strip spurious re-borrows when the argument is already a borrowed parameter.
    fn finalize_ir_call_arg(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        coerced: &str,
    ) -> String {
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) {
            if let Some(rest) = coerced.strip_prefix("&mut ") {
                return rest.to_string();
            }
            if let Some(rest) = coerced.strip_prefix('&') {
                return rest.to_string();
            }
            return coerced.to_string();
        }

        let Expression::Identifier { name, .. } = arg_expr else {
            if arg_str.ends_with(".clone()")
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                return coerced[1..].to_string();
            }
            return coerced.to_string();
        };
        if self.inferred_mut_borrowed_params.contains(name) && coerced.starts_with("&mut ") {
            return coerced["&mut ".len()..].to_string();
        }
        if (self.identifier_already_ref(name)
            || self.emitted_rust_ref_formals.contains(name)
            || self.borrowed_iterator_vars.contains(name))
            && !self.inferred_mut_borrowed_params.contains(name)
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            let is_copy = self
                .local_var_types
                .get(name)
                .is_some_and(|t| self.is_type_copy(t))
                || self
                    .infer_expression_type(arg_expr)
                    .is_some_and(|t| self.is_type_copy(&t));
            if is_copy {
                return format!("*{}", &coerced[1..]);
            }
            return coerced[1..].to_string();
        }
        coerced.to_string()
    }

    /// Infer the safety type of a call-site argument from solver-resolved types,
    /// parameter borrow state, generated Rust text, and expression shape (fallback).
    pub(crate) fn infer_actual_safety_type(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
    ) -> SafetyType {
        if arg_str.ends_with(".clone()") {
            let base = self
                .infer_expression_type(arg_expr)
                .as_ref()
                .map(|ty| crate::ir::node::parser_type_to_base_type(ty))
                .unwrap_or(BaseType::Inferred);
            return SafetyType::owned(base);
        }

        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) {
            return SafetyType::borrowed(BaseType::String, Region::fresh(2));
        }

        if let Expression::Identifier { name, .. } = arg_expr {
            if self.borrowed_iterator_vars.contains(name) {
                let base = self
                    .infer_expression_type(arg_expr)
                    .as_ref()
                    .map(|ty| match ty {
                        Type::Reference(inner) | Type::MutableReference(inner) => {
                            crate::ir::node::parser_type_to_base_type(inner)
                        }
                        other => crate::ir::node::parser_type_to_base_type(other),
                    })
                    .unwrap_or(BaseType::Inferred);
                return SafetyType::borrowed(base, Region::fresh(11));
            }
            // Match-arm bindings are owned enum/struct payloads even when
            // `local_var_types` temporarily marks them as references.
            if self.match_arm_bindings.contains(name.as_str()) {
                let base = self
                    .infer_expression_type(arg_expr)
                    .as_ref()
                    .map(|ty| match ty {
                        Type::Reference(inner) | Type::MutableReference(inner) => {
                            crate::ir::node::parser_type_to_base_type(inner)
                        }
                        other => crate::ir::node::parser_type_to_base_type(other),
                    })
                    .unwrap_or(BaseType::Inferred);
                return SafetyType::owned(base);
            }
            if self.inferred_mut_borrowed_params.contains(name) {
                return self.safety_type_for_param_binding(arg_expr, OwnedType::MutRef(Region::fresh(1)));
            }
            if self.identifier_already_ref(name) {
                return self.safety_type_for_param_binding(arg_expr, OwnedType::Ref(Region::fresh(0)));
            }
            // Forward-ref / owned formals: analyzer may still infer `&str` while Rust
            // emits `String` — coerce as owned at call sites (`check(&text)`).
            if self.current_function_params.iter().any(|p| p.name == *name)
                && !self.emitted_rust_ref_formals.contains(name)
            {
                if let Some(ty) = self.infer_expression_type(arg_expr) {
                    if matches!(
                        &ty,
                        Type::Reference(inner) | Type::MutableReference(inner)
                            if crate::codegen::rust::types::is_windjammer_text_type(inner.as_ref())
                    ) || crate::codegen::rust::types::is_windjammer_text_type(&ty)
                    {
                        return SafetyType::owned(BaseType::String);
                    }
                }
            }
        }

        if let Some(ownership) = ownership_from_rust_expr(arg_str) {
            let base = self
                .infer_expression_type(arg_expr)
                .as_ref()
                .map(|ty| crate::ir::node::parser_type_to_base_type(ty))
                .unwrap_or(BaseType::Inferred);
            return SafetyType {
                base,
                ownership,
                effects: crate::ir::safety_type::EffectSet::pure(),
                taint: crate::ir::safety_type::TaintStatus::Clean,
                const_eval: crate::ir::safety_type::ConstEval::Runtime,
                exec_mode: None,
            };
        }

        if let Some(ty) = self.infer_expression_type(arg_expr) {
            return safety_type_from_parser_type(&ty, None);
        }

        safety_type_from_arg_expression(arg_expr)
    }

    fn safety_type_for_param_binding(
        &self,
        arg_expr: &Expression<'ast>,
        ownership: OwnedType,
    ) -> SafetyType {
        let base = self
            .infer_expression_type(arg_expr)
            .as_ref()
            .map(|ty| type_pointee_base(ty))
            .unwrap_or(BaseType::Inferred);
        SafetyType {
            base,
            ownership,
            effects: crate::ir::safety_type::EffectSet::pure(),
            taint: crate::ir::safety_type::TaintStatus::Clean,
            const_eval: crate::ir::safety_type::ConstEval::Runtime,
            exec_mode: None,
        }
    }

    /// True when a callee param only field-extracts the argument (partial move semantics).
    fn callee_param_field_extracts(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        _arg_name: &str,
    ) -> bool {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let sig = registry
            .get_signature(callee_name)
            .or_else(|| registry.lookup_method(callee_name))
            .or_else(|| registry.find_signature_ending_with(simple))
            .or_else(|| {
                self.global_signature_registry.as_ref().and_then(|g| {
                    g.get_signature(callee_name)
                        .or_else(|| g.lookup_method(callee_name))
                        .or_else(|| g.find_signature_ending_with(simple))
                })
            });
        let Some(sig) = sig else {
            return false;
        };
        let param_idx = sig.arg_param_index(arg_index);
        sig.field_extract_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
            .unwrap_or(false)
    }

    /// Apply auto-clone analysis before IR coercion when a binding is reused after partial move.
    pub(crate) fn maybe_auto_clone_call_arg(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        callee_name: Option<&str>,
        arg_index: Option<usize>,
    ) -> String {
        match arg_expr {
            Expression::Identifier { name, .. } => {
                if self.param_used_in_prior_field_extract_call(name) {
                    return arg_str.to_string();
                }
                if let (Some(callee), Some(idx)) = (callee_name, arg_index) {
                    if self.callee_param_field_extracts_by_name(callee, idx) {
                        return arg_str.to_string();
                    }
                    if self.ir_callee_arg_expects_mut_borrow(
                        &self.signature_registry,
                        callee,
                        idx,
                        None,
                        None,
                    ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                        self.ir_callee_arg_expects_mut_borrow(g, callee, idx, None, None)
                    }) {
                        return arg_str.to_string();
                    }
                }
                self.maybe_auto_clone(name, arg_str)
            }
            Expression::FieldAccess { .. } | Expression::Index { .. } => {
                if let (Some(callee), Some(idx)) = (callee_name, arg_index) {
                    if self.callee_param_field_extracts_by_name(callee, idx) {
                        return arg_str.to_string();
                    }
                    if self.ir_callee_arg_expects_mut_borrow(
                        &self.signature_registry,
                        callee,
                        idx,
                        None,
                        None,
                    ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                        self.ir_callee_arg_expects_mut_borrow(g, callee, idx, None, None)
                    }) {
                        return arg_str.to_string();
                    }
                }
                self.maybe_auto_clone_expr_path(arg_expr, arg_str)
            }
            _ => arg_str.to_string(),
        }
    }

    /// Clone a field/index path when auto-clone analysis recorded a move+reuse site.
    pub(crate) fn maybe_auto_clone_expr_path(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
    ) -> String {
        if arg_str.ends_with(".clone()") || arg_str.starts_with('*') {
            return arg_str.to_string();
        }
        let Some(path) = Self::auto_clone_expr_path(arg_expr) else {
            return arg_str.to_string();
        };
        let needs = self.auto_clone_analysis.as_ref().is_some_and(|a| {
            a.needs_clone(&path, self.current_statement_idx).is_some()
                || a.needs_clone_anywhere(&path)
        });
        if needs {
            format!("{arg_str}.clone()")
        } else {
            arg_str.to_string()
        }
    }

    fn auto_clone_expr_path(expr: &Expression<'ast>) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                Some(format!("{}.{}", Self::auto_clone_expr_path(object)?, field))
            }
            Expression::Index { object, index, .. } => {
                let base = Self::auto_clone_expr_path(object)?;
                let index_str = match index {
                    Expression::Literal {
                        value: Literal::Int(n),
                        ..
                    } => n.to_string(),
                    Expression::Identifier { name, .. } => name.clone(),
                    _ => "*".to_string(),
                };
                Some(format!("{base}[{index_str}]"))
            }
            _ => None,
        }
    }

    fn ir_sig_arg_expects_shared_borrow(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> bool {
        let pidx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return false;
        }
        if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, pidx) {
            return true;
        }
        if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
            sig, pidx,
        ) {
            return false;
        }
        sig.param_types.get(pidx).is_some_and(|t| matches!(t, Type::Reference(_)))
            || matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                    sig, arg_index,
                ),
                crate::analyzer::OwnershipMode::Borrowed
            )
    }

    fn ir_callee_arg_expects_shared_borrow(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        user_arg_count: Option<usize>,
        local_sig: Option<&crate::analyzer::FunctionSignature>,
    ) -> bool {
        // Prefer emitted owned contracts over stale Borrowed analyzer/global stubs.
        if self.ir_callee_arg_emits_owned_contract(
            registry,
            callee_name,
            arg_index,
            user_arg_count,
            local_sig,
        ) {
            return false;
        }
        if let Some(sig) = local_sig {
            if self.ir_sig_arg_expects_shared_borrow(sig, arg_index) {
                return true;
            }
        }
        if let Some(sig) = registry.get_signature(callee_name) {
            if self.ir_sig_arg_expects_shared_borrow(sig, arg_index) {
                return true;
            }
        }
        if let Some(global) = self.global_signature_registry.as_ref() {
            if let Some(sig) = global.get_signature(callee_name) {
                if self.ir_sig_arg_expects_shared_borrow(sig, arg_index) {
                    return true;
                }
            }
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            if simple != callee_name {
                if let Some(sig) = global.get_signature(simple) {
                    if self.ir_sig_arg_expects_shared_borrow(sig, arg_index) {
                        return true;
                    }
                }
            }
        }
        if let Some((rt, method)) = callee_name.rsplit_once("::") {
            let arg_count = user_arg_count.unwrap_or(arg_index + 1);
            if self.method_registry_arg_expects_shared_borrow(rt, method, arg_index, arg_count) {
                return true;
            }
        }
        false
    }

    /// True when any resolved signature emits an owned (non-`&T`) formal for this arg.
    fn ir_callee_arg_emits_owned_contract(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        user_arg_count: Option<usize>,
        local_sig: Option<&crate::analyzer::FunctionSignature>,
    ) -> bool {
        let check = |sig: &crate::analyzer::FunctionSignature| {
            let pidx = sig.arg_param_index(arg_index);
            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
        };
        if local_sig.is_some_and(check) {
            return true;
        }
        if registry.get_signature(callee_name).is_some_and(check) {
            return true;
        }
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        if simple != callee_name && registry.get_signature(simple).is_some_and(check) {
            return true;
        }
        if let Some(global) = self.global_signature_registry.as_ref() {
            if global.get_signature(callee_name).is_some_and(check) {
                return true;
            }
            if simple != callee_name && global.get_signature(simple).is_some_and(check) {
                return true;
            }
        }
        if let Some((rt, method)) = callee_name.rsplit_once("::") {
            let arg_count = user_arg_count.unwrap_or(arg_index + 1);
            if let Some(sig) = self.resolve_method_function_signature(rt, method, arg_count) {
                if check(&sig) {
                    return true;
                }
            }
        }
        false
    }

    fn ir_sig_arg_expects_mut_borrow(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> bool {
        let pidx = sig.arg_param_index(arg_index);
        sig.param_types.get(pidx).is_some_and(|t| matches!(t, Type::MutableReference(_)))
            || matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                    sig, arg_index,
                ),
                crate::analyzer::OwnershipMode::MutBorrowed,
            )
    }

    fn ir_callee_arg_expects_mut_borrow(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        user_arg_count: Option<usize>,
        local_sig: Option<&crate::analyzer::FunctionSignature>,
    ) -> bool {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        if self
            .function_emitted_mut_arg_indices
            .get(callee_name)
            .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
            .is_some_and(|indices| indices.contains(&arg_index))
        {
            return true;
        }
        if let Some(sig) = local_sig {
            if self.ir_sig_arg_expects_mut_borrow(sig, arg_index) {
                return true;
            }
        }
        if let Some(sig) = registry.get_signature(callee_name) {
            if self.ir_sig_arg_expects_mut_borrow(sig, arg_index) {
                return true;
            }
        }
        if let Some(global) = self.global_signature_registry.as_ref() {
            if let Some(sig) = global.get_signature(callee_name) {
                if self.ir_sig_arg_expects_mut_borrow(sig, arg_index) {
                    return true;
                }
            }
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            if simple != callee_name {
                if let Some(sig) = global.get_signature(simple) {
                    if self.ir_sig_arg_expects_mut_borrow(sig, arg_index) {
                        return true;
                    }
                }
            }
        }
        if let Some((rt, method)) = callee_name.rsplit_once("::") {
            let arg_count = user_arg_count.unwrap_or(arg_index + 1);
            if let Some(sig) = self.resolve_method_function_signature(rt, method, arg_count) {
                if self.ir_sig_arg_expects_mut_borrow(&sig, arg_index) {
                    return true;
                }
            }
        }
        false
    }
}

/// Detect ownership already encoded in generated Rust (e.g. prior phases emitted `&x`).
fn ownership_from_rust_expr(expr: &str) -> Option<OwnedType> {
    if expr.starts_with("&mut ") {
        Some(OwnedType::MutRef(Region::fresh(1)))
    } else if expr.starts_with('&') {
        Some(OwnedType::Ref(Region::fresh(0)))
    } else {
        None
    }
}

fn type_pointee_base(ty: &Type) -> BaseType {
    match ty {
        Type::Reference(inner) | Type::MutableReference(inner) => {
            crate::ir::node::parser_type_to_base_type(inner)
        }
        other => crate::ir::node::parser_type_to_base_type(other),
    }
}

fn safety_type_from_arg_expression(expr: &Expression) -> SafetyType {
    match expr {
        Expression::Literal { value, .. } => match value {
            Literal::String(_) => SafetyType::borrowed(BaseType::String, Region::fresh(2)),
            Literal::Int(_) => SafetyType::copy(BaseType::I32),
            Literal::Float(_) => SafetyType::copy(BaseType::F32),
            Literal::Bool(_) => SafetyType::copy(BaseType::Bool),
            _ => SafetyType::owned(BaseType::Inferred),
        },
        Expression::Identifier { name, .. } => {
            if name.starts_with('"') {
                SafetyType::borrowed(BaseType::String, Region::fresh(2))
            } else {
                SafetyType::owned(BaseType::Inferred)
            }
        }
        Expression::FieldAccess { .. } => {
            SafetyType::borrowed(BaseType::Inferred, Region::fresh(3))
        }
        _ => SafetyType::owned(BaseType::Inferred),
    }
}
