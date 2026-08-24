//! IR-driven call-site argument coercion for Rust codegen.
//!
//! When `IrCutoverConfig.call_sites` is enabled, applies `encode_call_argument`
//! using callee signature expectations instead of heuristic borrow passes.

use crate::analyzer::SignatureRegistry;
use crate::codegen::rust::generator::CodeGenerator;
use crate::ir::coercion::compute_coercion;
use crate::ir::coercion::CoercionKind;
use crate::ir::safety_type::{BaseType, OwnedType, Region, SafetyType};
use crate::ir::signature_bridge::{safety_type_from_parser_type, safety_type_from_signature_param};
use crate::ir::target_encodings::{apply_coercion, Target};
use crate::parser::{Expression, Literal, Statement, Type};

impl<'ast> CodeGenerator<'ast> {
    fn refreshed_call_site_sig_for_arg<'a>(
        &self,
        registry: &'a SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
    ) -> crate::analyzer::FunctionSignature {
        crate::codegen::rust::signature_promotion::refresh_call_site_signature_for_arg(
            Some(sig.clone()),
            callee_name,
            arg_index,
            self.global_signature_registry.as_deref(),
            registry,
        )
        .unwrap_or_else(|| sig.clone())
    }

    /// Apply IR-driven coercion to a call-site argument when call_sites cutover is on.
    ///
    /// For known callees this is total: always returns `Some` when `call_sites` is on.
    /// Missing signatures at module boundaries are recorded as hard errors (fail closed).
    pub(crate) fn apply_ir_call_site_coercion(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        local_sig: Option<&crate::analyzer::FunctionSignature>,
        receiver_type_name: Option<&str>,
        user_arg_count: Option<usize>,
    ) -> Option<String> {
        if !self.ir_cutover.call_sites {
            return None;
        }

        // User-written `&x` / `&mut x`: preserve only when callee expects a borrow.
        // Owned formals need IR coercion (clone/deref), not passthrough.
        if crate::codegen::rust::expression_helpers::is_reference_expression(arg_expr)
            || ownership_from_rust_expr(arg_str).is_some()
        {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            let early_sig = registry
                .get_signature(callee_name)
                .cloned()
                .or_else(|| registry.lookup_method(callee_name).cloned())
                .or_else(|| {
                    local_sig
                        .cloned()
                        .filter(|local| local.emitted_rust_ref_params.is_none())
                })
                .or_else(|| local_sig.cloned())
                .or_else(|| {
                    receiver_type_name.and_then(|rt| {
                        self.resolve_method_function_signature(
                            rt,
                            simple,
                            user_arg_count.unwrap_or(arg_index + 1),
                        )
                    })
                });
            if let Some(ref sig) = early_sig {
                let pidx = sig.arg_param_index(arg_index);
                let wants_borrow = sig.param_types.get(pidx).is_some_and(|t| {
                    matches!(t, Type::Reference(_) | Type::MutableReference(_))
                }) || matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, arg_index,
                    ),
                    crate::analyzer::OwnershipMode::Borrowed | crate::analyzer::OwnershipMode::MutBorrowed,
                ) || sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(pidx))
                    .copied()
                    == Some(true);
                if wants_borrow {
                    return Some(arg_str.to_string());
                }
            }
        }

        // Never skip auto-clone when the callee emits owned formals: analyzer/global
        // stubs may still say Borrowed (`&Vec`) while codegen emits `Vec` (regression-056/059).
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
        if (self.borrowed_iterator_vars.contains(name)
            || self.local_var_types.get(name).is_some_and(|t| {
                matches!(t, Type::Reference(_) | Type::MutableReference(_))
            }))
        && crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
            self.current_function_return_type.as_ref(),
        ));
        let mut prepared_arg = match arg_expr {
            Expression::Identifier { .. }
                if !skip_auto_clone_for_borrow
                    && !skip_auto_clone_for_field_extract
                    && !collecting_ref_vec =>
            {
                self.maybe_auto_clone_call_arg(
                    arg_expr,
                    arg_str,
                    Some(callee_name),
                    Some(arg_index),
                )
            }
            // Field paths (`record.key`) moved into owned formals + reused in loops
            // need `.clone()`; identifier-only auto-clone misses them (regression-059).
            Expression::FieldAccess { .. } | Expression::Index { .. }
                if !skip_auto_clone_for_borrow && !skip_auto_clone_for_field_extract =>
            {
                self.maybe_auto_clone_call_arg(
                    arg_expr,
                    arg_str,
                    Some(callee_name),
                    Some(arg_index),
                )
            }
            _ => arg_str.to_string(),
        };

        let method_simple = callee_name.rsplit("::").next().unwrap_or(callee_name);

        // Module-qualified free calls without an exact registry key: fail closed.
        // Do not coerce from cross-module homonyms or guess ownership. Inline `mod`
        // callees may register only the bare name — allow that fallback.
        // Method calls supply a receiver type — never treat them as module boundaries
        // even if a buggy callee key looks like `local_var::method`.
        if receiver_type_name.is_none()
            && Self::is_module_boundary_callee(callee_name)
            && !crate::codegen::rust::stdlib_method_traits::is_runtime_std_module(
                crate::codegen::rust::stdlib_method_traits::runtime_module_segment_from_callee_path(
                    callee_name,
                ),
            )
        {
            let has_exact_module_sig = registry.get_signature(callee_name).is_some()
                || self
                    .global_signature_registry
                    .as_ref()
                    .is_some_and(|g| g.get_signature(callee_name).is_some());
            let has_inline_simple_sig = self.inline_module_qualified_call(callee_name)
                && (registry.get_signature(method_simple).is_some()
                    || self
                        .global_signature_registry
                        .as_ref()
                        .is_some_and(|g| g.get_signature(method_simple).is_some())
                    || local_sig.is_some());
            if !has_exact_module_sig && !has_inline_simple_sig {
                self.report_missing_boundary_signature(callee_name);
                return Some(prepared_arg);
            }
        }

        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let mut sig = if receiver_type_name.is_none() && !callee_name.contains("::") {
            let from_global = self
                .global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(callee_name).cloned());
            let from_global_simple = self
                .global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(simple).cloned());
            let from_reg = registry.get_signature(callee_name).cloned();
            let from_local = local_sig.cloned();
            // Prefer defining-module / global refresh first so cross-module free calls
            // see `&str` (regression-049 `replay_to_lsn`) over stale local owned stubs.
            crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                from_global,
                from_global_simple,
                from_reg,
                from_local,
            ])
        } else {
            let from_local = local_sig.cloned();
            let from_reg = registry.get_signature(callee_name).cloned();
            let from_global = self
                .global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(callee_name).cloned());
            let from_simple = if self.inline_module_qualified_call(callee_name) {
                registry.get_signature(simple).cloned().or_else(|| {
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned())
                })
            } else {
                None
            };
            crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                from_global,
                from_reg,
                from_simple,
                from_local,
            ])
        }
        .or_else(|| registry.lookup_method(callee_name).cloned());
        let sig_for_owned_literal = sig.clone();
        // Upgrade stale owned WJ `string` formals to defining-module `&str` emission.
        // Never consult bare method-name keys for type-qualified calls (`App::record_resource`)
        // — homonyms can replace the whole signature with unrelated Borrowed formals.
        if let Some(ref base) = sig {
            let pidx = base.arg_param_index(arg_index);
            let mut upgraded = sig.clone();
            let type_qualified =
                crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                    callee_name,
                );
            let challengers: Vec<Option<&crate::analyzer::FunctionSignature>> = if type_qualified {
                vec![
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(callee_name)),
                    registry.get_signature(callee_name),
                ]
            } else {
                vec![
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(callee_name)),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple)),
                    registry.get_signature(callee_name),
                    registry.get_signature(simple),
                ]
            };
            for challenger in challengers {
                upgraded =
                    crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                        upgraded, challenger, pidx,
                    );
            }
            sig = upgraded;
        }

        if let Some((receiver_ty, method)) = callee_name.rsplit_once("::") {
            // Only Type::method (uppercase receiver). Runtime modules (`strings::join`)
            // and lowercase module paths must keep the free-function / prefer-shared sig —
            // resolving `strings` as a type picks Vec::join-style owned contracts and
            // forces `parts.clone()` into `&[String]` formals (flat lib.wj / wj test).
            if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                callee_name,
            ) {
                let arg_count = user_arg_count.unwrap_or(arg_index + 1);
                let inferred_recv = receiver_type_name.and_then(|rt| {
                    crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                        Some(rt),
                        None,
                        self.current_function_return_type.as_ref(),
                    )
                });
                if let Some(method_sig) = self.resolve_method_function_signature_specialized(
                    receiver_ty,
                    method,
                    arg_count,
                    inferred_recv.as_ref(),
                ) {
                    let prefer_method = sig.as_ref().is_none_or(|local| {
                        let local_idx = local.arg_param_index(arg_index);
                        let method_idx = method_sig.arg_param_index(arg_index);
                        // Keep local owned / Copy-aggregate contracts over stale method Ref wraps
                        // (regression-060 `other: Lsn` must not become `&through` via method_ref prefer).
                        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            local, local_idx,
                        ) {
                            return false;
                        }
                        if local.formal_param_type(local_idx).is_some_and(|t| {
                            let bare = match t {
                                Type::Reference(inner) | Type::MutableReference(inner) => {
                                    inner.as_ref()
                                }
                                other => other,
                            };
                            self.is_type_copy(bare)
                                && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                        }) {
                            return false;
                        }
                        let local_ref = local
                            .param_types
                            .get(local_idx)
                            .is_some_and(|t| matches!(t, Type::Reference(_)));
                        let method_idx = method_sig.arg_param_index(arg_index);
                        let copy_aggregate_method = method_sig
                            .formal_param_type(method_idx)
                            .is_some_and(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                            });
                        let method_emits_shared = crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            &method_sig, method_idx,
                        );
                        let local_owned = matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                                local, local_idx,
                            ),
                            crate::analyzer::OwnershipMode::Owned
                        );
                        (method_emits_shared && !local_ref && !copy_aggregate_method)
                            || (local_owned && method_emits_shared && !copy_aggregate_method)
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
        }

        if let Some(global) = self.global_signature_registry.as_ref() {
            if let Some(global_sig) = global.get_signature(callee_name) {
                let global_idx = global_sig.arg_param_index(arg_index);
                let method_registry_owned = crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                    callee_name,
                ) && callee_name.rsplit_once("::").is_some_and(
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
                        global_sig, global_idx,
                    )
                    && sig.as_ref().is_none_or(|local_sig| {
                        let idx = local_sig.arg_param_index(arg_index);
                        // Never replace codegen-owned / Copy-aggregate formals with a stale
                        // global Borrowed wrap (regression-060 `is_at_or_before(&through)` vs `other: Lsn`).
                        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            local_sig, idx,
                        ) {
                            return false;
                        }
                        if local_sig.formal_param_type(idx).is_some_and(|t| {
                            let bare = match t {
                                Type::Reference(inner) | Type::MutableReference(inner) => {
                                    inner.as_ref()
                                }
                                other => other,
                            };
                            self.is_type_copy(bare)
                                && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                        }) {
                            return false;
                        }
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
                        let local_owned =
                            matches!(local_eff, crate::analyzer::OwnershipMode::Owned);
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

        let Some(mut sig) = sig else {
            // Unresolved non-boundary callees (stdlib Pattern, bare names): finish
            // via runtime/registry helpers — never name-based ownership guesses.
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
            // Unresolved stdlib Pattern/`&str` methods still need `&needle` when the
            // registry knows the formal is `&str` (e.g. `String::find` in stdlib_meta).
            if !finished.starts_with('&')
                && crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
                    method_simple,
                    receiver_type_name.or(Some("String")),
                    registry,
                    arg_index,
                )
                && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_expr)
            {
                crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                    arg_expr,
                    &mut finished,
                );
                if !finished.starts_with('&')
                    && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                {
                    finished = crate::ir::target_encodings::rust_shared_borrow(&finished);
                }
            }
            // Unresolved callee still needs multi-use clones (regression-063 seed_write).
            finished = self.maybe_auto_clone_call_arg(
                arg_expr,
                &finished,
                Some(callee_name),
                Some(arg_index),
            );
            if matches!(
                arg_expr,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            ) && crate::codegen::rust::string_utilities::type_qualified_associated_string_literal_needs_rust_owned_string(
                callee_name,
                arg_index,
                None,
                registry,
                self.global_signature_registry.as_deref(),
            ) && !crate::codegen::rust::string_utilities::already_owned_string_expr(&finished)
            {
                finished = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                    finished.trim_start_matches('&'),
                );
            }
            return Some(finished);
        };

        // Specialize stdlib generics (`Vec::push(T)` → `push(String)` on `Vec<String>`).
        if let Some(recv_ty) =
            crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                receiver_type_name,
                None,
                self.current_function_return_type.as_ref(),
            )
        {
            crate::codegen::rust::stdlib_signature_specialization::specialize_signature_for_receiver(
                &mut sig, &recv_ty,
            );
        }

        // Unified local+global resolution (same as `mc_resolve_method_call_signature`) so
        // stale per-caller registry stubs do not beat converged defining-module metadata.
        if let Some(rt) = receiver_type_name {
            if let Some(resolved) =
                crate::codegen::rust::call_signature_resolution::resolve_method_for_call_site(
                    registry,
                    self.global_signature_registry.as_deref(),
                    rt,
                    method_simple,
                    user_arg_count.unwrap_or(arg_index + 1),
                )
            {
                sig = resolved.sig;
            }
            // Fallback: per-caller registry snapshots can predate defining-module
            // convergence; trust the merged global entry when it expects shared borrow.
            // Never downgrade codegen-owned Copy-aggregate locals (regression-060
            // `is_at_or_before(&through)` vs emitted `other: Lsn`). Non-Copy owned
            // formals (Key::has_key) may still need global convergence for clone paths.
            if let Some(global) = self.global_signature_registry.as_deref() {
                let qualified = format!("{rt}::{method_simple}");
                if let Some(gs) = global.get_signature(&qualified) {
                    let gidx = gs.arg_param_index(arg_index);
                    if crate::ir::signature_bridge::call_site_expects_shared_borrow(gs, gidx) {
                        let pidx = sig.arg_param_index(arg_index);
                        let local_copy_aggregate_owned = sig
                            .formal_param_type(pidx)
                            .is_some_and(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                            })
                            && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                &sig, pidx,
                            )
                                || sig
                                    .emitted_rust_ref_params
                                    .as_ref()
                                    .and_then(|flags| flags.get(pidx))
                                    .copied()
                                    != Some(true));
                        if !crate::ir::signature_bridge::call_site_expects_shared_borrow(&sig, pidx)
                            && !local_copy_aggregate_owned
                        {
                            sig = gs.clone();
                        }
                    }
                }
            }
        }

        // Method callees: always attempt defining-module codegen refresh merge so
        // `emitted_rust_ref_params` / owned Copy-aggregate formals beat stale analyzer
        // `Reference(Lsn)` stubs (`&through` into `other: Lsn`, regression-060). No-op when the
        // registry lacks refresh metadata.
        if let Some(rt) = receiver_type_name {
            let qualified = format!("{rt}::{method_simple}");
            let refresh_keys = vec![qualified, callee_name.to_string()];
            crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                &mut sig,
                registry,
                &refresh_keys,
            );
            if let Some(global) = self.global_signature_registry.as_ref() {
                crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                    &mut sig,
                    global,
                    &refresh_keys,
                );
            }
        }

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

        let receiver_is_set = receiver_type_name
            .is_some_and(crate::codegen::rust::stdlib_method_traits::is_set_type_name);

        // Refresh free-fn signatures from the codegen registry before expected-type /
        // coercion decisions — analyzer stubs often still say bare `string`+Owned while
        // the defining-fn refresh recorded `&str` (`process("hello")` must stay bare).
        // Mirror method path: merge defining-module `emitted_rust_ref_params` so owned
        // Custom formals (`csr: DenseCsr`) beat WDB-097 cold-meta Borrowed stubs.
        // Skip type-qualified associated calls (`ServerResponse::error`) — bare `error`
        // would merge `log::error`'s `&str` formal onto owned i64 status.
        if receiver_type_name.is_none()
            && !crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                callee_name,
            )
        {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            let refresh_keys = vec![callee_name.to_string(), simple.to_string()];
            crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                &mut sig,
                registry,
                &refresh_keys,
            );
            if let Some(global) = self.global_signature_registry.as_ref() {
                crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                    &mut sig,
                    global,
                    &refresh_keys,
                );
                for key in [callee_name, simple] {
                    if let Some(reg) = global.lookup_method(key) {
                        if reg.emitted_rust_ref_params.is_some() {
                            crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                                &mut sig,
                                reg,
                            );
                            break;
                        }
                    }
                }
            }
            if let Some(refreshed) =
                crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(callee_name).cloned()),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned()),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.lookup_method(callee_name).cloned()),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.lookup_method(simple).cloned()),
                    self.signature_registry.get_signature(callee_name).cloned(),
                    self.signature_registry.get_signature(simple).cloned(),
                    self.signature_registry.lookup_method(callee_name).cloned(),
                    self.signature_registry.lookup_method(simple).cloned(),
                    Some(sig.clone()),
                ])
            {
                let ridx = refreshed.arg_param_index(arg_index);
                if refreshed.emitted_rust_ref_params.is_some()
                    || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        &refreshed, ridx,
                    )
                    || refreshed.param_types.get(ridx).is_some_and(|t| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    })
                    || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &refreshed, ridx,
                    )
                {
                    sig = refreshed;
                }
            }
        }

        // Final associated-call refresh: importer stubs may carry all-false
        // `emitted_rust_ref_params` while the defining module published `[true]`.
        if let Some(refreshed) =
            crate::codegen::rust::signature_promotion::refresh_call_site_signature_for_arg(
                if let Some(rt) = receiver_type_name {
                    self.resolve_method_function_signature(
                        rt,
                        method_simple,
                        user_arg_count.unwrap_or(arg_index + 1),
                    )
                } else {
                    None
                },
                callee_name,
                arg_index,
                self.global_signature_registry.as_deref(),
                registry,
            )
        {
            sig = refreshed;
        }
        // Body-converged `&str` refresh must not undo trait owned `string` contracts
        // (`authenticate(email: string)` → never `&request.email`).
        if let Some(global) = self.global_signature_registry.as_ref() {
            crate::codegen::rust::call_signature_resolution::apply_trait_owned_string_call_site_contracts(
                global,
                method_simple,
                &mut sig,
            );
            sig =
                crate::codegen::rust::call_signature_resolution::finalize_call_site_signature(sig);
        }
        crate::codegen::rust::signature_promotion::restore_stdlib_collection_key_contract(
            &mut sig,
            Some(callee_name),
        );

        let mut param_idx = sig.arg_param_index(arg_index);
        let mut expected = safety_type_from_signature_param(&sig, param_idx);
        if (crate::codegen::rust::signature_promotion::bare_formal_is_vec_or_map(&sig, param_idx)
            || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                &sig, param_idx,
            ))
            && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            && !crate::ir::signature_bridge::call_site_expects_shared_borrow(&sig, param_idx)
        {
            if let Some(bare) = crate::ir::signature_bridge::bare_wj_formal_type(&sig, param_idx) {
                expected = crate::ir::signature_bridge::safety_type_from_parser_type(
                    bare,
                    Some(crate::analyzer::OwnershipMode::Owned),
                );
            }
        }
        if std::env::var_os("WJ_DEBUG_COLLISION_BORROW").is_some() {
            eprintln!(
                "WJ_DEBUG_COLLISION_BORROW expected callee={callee_name} emitted={:?} \
                 param_ty={:?} expected_own={:?} callee_emits={}",
                sig.emitted_rust_ref_params,
                sig.param_types.get(param_idx),
                expected.ownership,
                crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &sig, param_idx,
                ),
            );
        }
        if collecting_ref_vec {
            if let Some(rt) = self.current_function_return_type.as_ref() {
                let elem_ref: Option<&Type> = match rt {
                    Type::Vec(inner) => Some(inner.as_ref()),
                    Type::Parameterized(name, args) if name == "Vec" && args.len() == 1 => {
                        args.first().map(|t| t as &Type)
                    }
                    _ => None,
                };
                if let Some(Type::Reference(inner) | Type::MutableReference(inner)) = elem_ref {
                    expected = safety_type_from_parser_type(
                        &Type::Reference(inner.clone()),
                        Some(crate::analyzer::OwnershipMode::Borrowed),
                    );
                }
            }
        }
        // Match-arm owned String payloads borrow as &str at *shared-ref* text callees.
        // Owned WJ `string` formals (`generate_page(markdown: string)`) must still move.
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.match_arm_bindings.contains(name.as_str()) {
                let expects_shared_text = crate::ir::signature_bridge::call_site_expects_shared_borrow(
                    &sig, param_idx,
                ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &sig, param_idx,
                ) || sig.param_types.get(param_idx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                });
                if expects_shared_text {
                    expected.base = BaseType::String;
                    expected.ownership = OwnedType::Ref(Region::fresh(12));
                }
            }
        }
        let callee_module = crate::codegen::rust::stdlib_method_traits::resolve_runtime_std_module(
            callee_name.split("::").next().unwrap_or(""),
            receiver_type_name,
        );
        let inferred_arg_type = self.infer_expression_type(arg_expr);
        let runtime_param_type = sig
            .formal_param_type(param_idx)
            .or(inferred_arg_type.as_ref());
        if let Some(formal_ty) = runtime_param_type {
            if crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                registry,
                callee_name,
                Some(&sig),
                arg_index,
            ) {
                if crate::codegen::rust::types::is_windjammer_text_type(formal_ty) {
                    expected.base = BaseType::String;
                }
                expected.ownership = OwnedType::Ref(Region::fresh(7));
            } else if crate::codegen::rust::string_utilities::param_is_rust_str_ref(formal_ty) {
                expected.base = BaseType::String;
                expected.ownership = OwnedType::Ref(Region::fresh(5));
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
            ) && crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                &sig, arg_index,
            ) {
                // Scanned runtime AsRef/&str formals: keep owned WJ text as Ref at call site.
                if crate::codegen::rust::types::is_windjammer_text_type(
                    sig.formal_param_type(param_idx)
                        .or_else(|| sig.param_types.get(param_idx))
                        .unwrap_or(&Type::String),
                ) || runtime_param_type
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                {
                    expected.base = BaseType::String;
                }
                expected.ownership = OwnedType::Ref(Region::fresh(8));
            }
        }
        if crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
            &sig,
            arg_index,
        ) && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_expr)
        {
            // `str::find` / `contains` / … take `Pattern` (`&str`, `char`, …).
            // String literals are already `&str`; owned `String` needles need `&`.
            expected.base = BaseType::String;
            expected.ownership = OwnedType::Ref(Region::fresh(9));
        } else if crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
            &sig,
            arg_index,
            receiver_type_name,
        ) || (arg_index == 0
            && receiver_is_set
            && crate::codegen::rust::stdlib_method_traits::method_arg_expects_borrowed_reference_from_sig(
                &sig, arg_index,
            ))
        {
            // Signature-driven: map/set lookup formals are `&T` (Borrowed), never method-name lists.
            let arg_already_borrowed = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.identifier_already_ref(name)
                        || self.inferred_borrowed_params.contains(name)
                        || self.emitted_rust_ref_formals.contains(name)
            );
            if !arg_already_borrowed {
                expected.ownership = OwnedType::Ref(Region::fresh(4));
            }
        } else if crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
            &sig, arg_index,
        ) && matches!(expected.base, BaseType::String | BaseType::Custom(_))
            && !crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                &sig, arg_index,
            )
        {
            expected.ownership = OwnedType::Owned;
        }
        // Registry-aware Copy aggregate → owned callee formal (regression-060 `through: Lsn`).
        if let Expression::Identifier { name, .. } = arg_expr {
            let caller_copy_aggregate = self.current_function_params.iter().any(|p| {
                p.name == *name
                    && self.is_type_copy(&p.type_)
                    && !crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
            });
            if caller_copy_aggregate {
                let callee_copy_owned =
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, param_idx,
                    ) || sig
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(param_idx))
                        .copied()
                        == Some(false)
                        || sig
                            .formal_param_type(param_idx)
                            .or_else(|| sig.param_types.get(param_idx))
                            .is_some_and(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                            });
                if callee_copy_owned
                    && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        &sig, param_idx,
                    )
                {
                    if let Some(bare) =
                        crate::ir::signature_bridge::bare_wj_formal_type(&sig, param_idx)
                    {
                        expected = crate::ir::signature_bridge::safety_type_from_parser_type(
                            bare,
                            Some(crate::analyzer::OwnershipMode::Owned),
                        );
                    }
                }
            }
        }
        let actual = self.infer_actual_safety_type(arg_expr, prepared_arg.as_str());
        let mut kind = compute_coercion(&actual, &expected);
        // `rows[i]` / `self.field` into owned non-Copy formals: clone when the root cannot
        // move (shared/`&mut self`, or WJ bare `self` that emits `&self`). Always cloning
        // `self.field` into Owned is correct for `&self` (E0507) and harmless for owned-self.
        let field_from_self = matches!(
            arg_expr,
            Expression::FieldAccess { object, .. }
                if matches!(&**object, Expression::Identifier { name, .. } if name == "self")
        );
        if matches!(expected.ownership, OwnedType::Owned)
            && !prepared_arg.ends_with(".clone()")
            && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            && (matches!(arg_expr, Expression::Index { .. })
                || (matches!(arg_expr, Expression::FieldAccess { .. })
                    && (self.field_access_root_is_behind_reference(arg_expr) || field_from_self)))
        {
            let elem_needs_clone = self.infer_expression_type(arg_expr).map_or_else(
                || matches!(expected.base, BaseType::Custom(_) | BaseType::String),
                |t| {
                    let bare = match &t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    !self.is_type_copy(bare)
                },
            );
            if elem_needs_clone {
                kind = CoercionKind::Clone;
            }
        }
        if matches!(kind, CoercionKind::ToOwnedString)
            && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
        {
            kind = CoercionKind::Identity;
        }
        if matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow) {
            if let Expression::Identifier { name, .. } = arg_expr {
                let caller_copy_aggregate = self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && self.is_type_copy(&p.type_)
                        && !crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
                });
                if caller_copy_aggregate {
                    let callee_copy_owned = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, param_idx,
                    ) || sig
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(param_idx))
                        .copied()
                        == Some(false)
                        || sig
                            .formal_param_type(param_idx)
                            .or_else(|| sig.param_types.get(param_idx))
                            .is_some_and(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                            });
                    if callee_copy_owned
                        && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            &sig, param_idx,
                        )
                    {
                        kind = CoercionKind::Identity;
                    }
                }
            }
        }
        if matches!(kind, CoercionKind::Clone)
            && matches!(expected.ownership, OwnedType::Ref(_))
            && crate::ir::coercion::is_string_base(&expected.base)
            && matches!(actual.ownership, OwnedType::Owned | OwnedType::Copy)
        {
            kind = CoercionKind::Borrow;
        }
        if crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
            && (crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                &sig, arg_index,
            ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
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
                    if self.ir_sig_arg_expects_shared_borrow(&sig, arg_index) {
                        kind = CoercionKind::Borrow;
                    } else {
                        kind = CoercionKind::Identity;
                    }
                }
            }
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            let runtime_needs_borrow =
                crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    registry,
                    callee_name,
                    Some(&sig),
                    arg_index,
                );
            if self.binding_emits_as_rust_shared_ref(name)
                && matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
                && !runtime_needs_borrow
            {
                kind = CoercionKind::Identity;
            }
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            let collects_ref_vec = crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                self.current_function_return_type.as_ref(),
            );
            let is_borrowed_loop_elem = self.borrowed_iterator_vars.contains(name)
                || (collects_ref_vec
                    && self.local_var_types.get(name).is_some_and(|t| {
                        matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    }));
            if is_borrowed_loop_elem && matches!(kind, CoercionKind::Clone) {
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
                let arg_is_current_fn_param =
                    self.current_function_params.iter().any(|p| p.name == *name);
                if this_arg_expects_borrow && arg_is_current_fn_param {
                    kind = CoercionKind::Identity;
                }
            }
        }
        // `.clone()` already produces an owned value — never prefix shared `&`.
        // `String` deref-coerces into `&str` formals (`foo(x.clone())`).
        if prepared_arg.ends_with(".clone()") && matches!(kind, CoercionKind::Borrow) {
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
        if matches!(kind, CoercionKind::Deref | CoercionKind::StripBorrow) {
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.copy_match_payload_binding(name)
                    && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
                {
                    kind = CoercionKind::Identity;
                }
            }
            // Copy field access is already a value in Rust (`failure.status` → i64).
            if matches!(arg_expr, Expression::FieldAccess { .. })
                && self.expression_is_copy(arg_expr)
                && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
            {
                kind = CoercionKind::Identity;
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
                .or_else(|| sig.param_types.get(param_idx))
                .is_some_and(|t| {
                    let bare = match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    self.is_type_copy(bare)
                        && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                });
            if callee_formal_is_copy {
                kind = CoercionKind::Identity;
            } else if let Expression::Unary { operand, .. } = arg_expr {
                if self
                    .infer_expression_type(operand)
                    .as_ref()
                    .is_some_and(|t| {
                        matches!(t, Type::Reference(inner) | Type::MutableReference(inner)
                        if self.is_type_copy(inner.as_ref()))
                    })
                {
                    kind = CoercionKind::Identity;
                }
            }
        }
        // Borrow coercion on `binding.clone()` / `self.field.clone()` → `&binding` /
        // `&self.field` (clone-before-borrow is redundant for shared-ref formals).
        if prepared_arg.ends_with(".clone()")
            && matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
            && matches!(
                arg_expr,
                Expression::Identifier { .. } | Expression::FieldAccess { .. }
            )
        {
            prepared_arg = prepared_arg.trim_end_matches(".clone()").to_string();
        }
        if matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
            && matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.match_arm_bindings.contains(name.as_str())
            )
            && !prepared_arg.starts_with('&')
        {
            let binding_is_copy = self
                .infer_expression_type(arg_expr)
                .is_some_and(|t| match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => {
                        self.is_type_copy(inner.as_ref())
                    }
                    other => self.is_type_copy(&other),
                });
            if !binding_is_copy {
                prepared_arg = format!("&{prepared_arg}");
            }
        }
        // Ambiguous `Type::method` signatures (two modules define `Emitter::new` with
        // different param types): do not auto-cast int→float from the winning sig.
        if matches!(kind, CoercionKind::NumericCast(_)) {
            let method = callee_name.rsplit("::").next().unwrap_or(callee_name);
            let type_name = receiver_type_name.or_else(|| {
                callee_name
                    .rsplit_once("::")
                    .map(|(q, _)| q)
                    .filter(|q| q.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
            });
            let qualified = type_name.map(|tn| format!("{tn}::{method}"));
            if self.should_skip_int_to_float_auto_cast_with_global(
                type_name,
                method,
                qualified.as_deref().or(Some(callee_name)),
            ) || self.has_collision_with_global(callee_name)
                || qualified
                    .as_ref()
                    .is_some_and(|k| self.has_collision_with_global(k))
            {
                kind = CoercionKind::Identity;
            }
        }
        let resolved_kind = kind;
        let coerced = apply_coercion(&resolved_kind, prepared_arg.as_str(), Target::Rust);
        let mut coerced = self.finalize_ir_call_arg(arg_expr, prepared_arg.as_str(), &coerced);

        // Unified borrow guard: registry Reference(T) / Borrowed ownership must reach call sites
        // even when IR coercion chose Identity (stale effective ownership metadata).
        let arg_already_rust_ref = matches!(
            arg_expr,
            Expression::Identifier { name, .. } if self.identifier_binding_already_rust_ref(name)
        );
        let formal_is_copy = sig
            .formal_param_type(param_idx)
            .or_else(|| sig.param_types.get(param_idx))
            .is_some_and(|t| {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                self.is_type_copy(bare)
                    && !crate::type_classification::is_copy_pass_by_value_formal(bare)
            });
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
        let arg_binding_already_rust_ref = matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.identifier_binding_already_rust_ref(name)
        );
        if arg_binding_already_rust_ref {
            borrow_decision.add_ref = false;
            borrow_decision.add_mut_ref = false;
            // IR Ref / shared auto-borrow may already have prefixed `&` onto an
            // emitted `&mut T` / `&T` binding (`take_in_edges(&csr)` → `&&mut`).
            coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                .to_string();
        }
        // Stale multipass metadata may infer borrow for plain `string` formals on user
        // free functions that actually emit owned `String` (circular-dep convergence).
        // Never suppress mut-borrow when the slot expects `&mut T`.
        if receiver_type_name.is_none()
            && crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn_with_global(
                &self.signature_registry,
                self.global_signature_registry.as_deref(),
                callee_name,
                &sig,
                param_idx,
                arg_index,
            )
        {
            borrow_decision.add_ref = false;
            if !self.ir_sig_arg_expects_mut_borrow(&sig, arg_index) {
                borrow_decision.add_mut_ref = false;
            }
        }
        // `.clone()` is already owned; `&str` formals deref-coerce from `String`.
        if coerced.ends_with(".clone()") {
            borrow_decision.add_ref = false;
        }
        crate::codegen::rust::call_site_borrow::apply_call_site_borrow(
            &borrow_decision,
            &mut coerced,
        );
        if coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
            && coerced.ends_with(".clone()")
        {
            coerced = coerced[1..].to_string();
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.in_user_written_closure
                && self.user_closure_params.contains(name)
                && self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
                && !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                coerced = format!("&{coerced}");
            }
        }
        if !(matches!(arg_expr, Expression::Identifier { name, .. }
            if self.in_user_written_closure && self.user_closure_params.contains(name))
            && self.ir_sig_arg_expects_shared_borrow(&sig, arg_index))
            && crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn_with_global(
            &self.signature_registry,
            self.global_signature_registry.as_deref(),
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
        ) && crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(
            arg_expr,
        ) && !coerced.starts_with("&mut ")
        {
            crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                arg_expr,
                &mut coerced,
                &self.current_function_params,
                &self.inferred_mut_borrowed_params,
            );
        }
        // Owned effective formals must not keep a stale `&` / `&mut` from earlier passes
        // (Copy aggregates and non-Copy deps like AppDeps emit `mut deps: AppDeps`).
        // Never peel when this slot expects mut-borrow (`fill_grid(grid: &mut VoxelGrid)`),
        // including when codegen already recorded the slot in `function_emitted_mut_arg_indices`
        // but a stale Owned refresh is what `sig` currently holds.
        let mut_arg_emitted = {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            self.function_emitted_mut_arg_indices
                .get(callee_name)
                .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
                .is_some_and(|indices| indices.contains(&arg_index))
        };
        let expects_mut_here = mut_arg_emitted
            || self.ir_sig_arg_expects_mut_borrow(&sig, arg_index)
            || self.ir_callee_arg_expects_mut_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                Some(&sig),
            );
        if !expects_mut_here {
            let peel_sig =
                self.refreshed_call_site_sig_for_arg(registry, callee_name, arg_index, &sig);
            let peel_pidx = peel_sig.arg_param_index(arg_index);
            if !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &peel_sig, peel_pidx,
            ) && !self.ir_sig_arg_expects_shared_borrow(&peel_sig, arg_index)
                && (crate::ir::signature_bridge::call_site_expects_owned_pass(&peel_sig, peel_pidx)
                    || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &peel_sig, peel_pidx,
                    ))
            {
                if coerced.starts_with("&mut ")
                    || (coerced.starts_with('&') && !coerced.starts_with("&mut "))
                {
                    coerced =
                        crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                            .to_string();
                }
            }
        } else if !expects_mut_here
            && matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                    &sig, arg_index,
                ),
                crate::analyzer::OwnershipMode::Owned,
            )
            && coerced.starts_with("&mut ")
        {
            coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced).to_string();
        }
        // Belt-and-suspenders: owned Copy aggregates pass by value at call sites.
        // Use registry-aware `is_type_copy` (Lsn, PartId, …) — pure analysis only knows
        // primitives and would miss user Copy aggregates (regression-060 `is_at_or_before`).
        // Do not strip `&mut` when the callee emits `&mut T` (Copy + MutBorrowed PlayerState).
        if coerced.starts_with("&mut ")
            || (coerced.starts_with('&') && !coerced.starts_with("&mut "))
        {
            let callee_bare = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
                .map(|t| match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                });
            if let Some(bare) = callee_bare {
                if self.is_type_copy(bare)
                    && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                    && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
                    && !self.ir_sig_arg_expects_mut_borrow(&sig, arg_index)
                {
                    coerced =
                        crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                            .to_string();
                }
            }
        }

        // Method-registry / global converged signatures must win over stale call-site
        // metadata (dogfood `engine.put` delegation, forward refs to later impl methods).
        self.apply_registry_borrow_to_call_arg(
            &mut coerced,
            arg_expr,
            receiver_type_name,
            method_simple,
            arg_index,
            user_arg_count,
        );

        // `apply_registry_borrow_to_call_arg` may re-apply stale `&` from global stubs;
        // Copy-aggregate caller→callee must stay by-value (regression-060).
        self.peel_copy_aggregate_caller_into_owned_callee(
            &mut coerced,
            arg_expr,
            &sig,
            arg_index,
            receiver_type_name,
        );

        // Final owned-contract enforcement after registry re-borrow (dogfood), …)` while formal emits owned `mut deps: AppDeps`).
        // Never strip when this slot is a confirmed shared-ref formal (`&str` / `&T`).
        // Never strip `&mut` when the slot expects mut-borrow.
        let expects_mut = matches!(
            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                &sig, arg_index,
            ),
            crate::analyzer::OwnershipMode::MutBorrowed,
        ) || sig
            .param_types
            .get(param_idx)
            .is_some_and(|t| matches!(t, Type::MutableReference(_)));
        let peel_sig = self.refreshed_call_site_sig_for_arg(registry, callee_name, arg_index, &sig);
        let peel_pidx = peel_sig.arg_param_index(arg_index);
        if !expects_mut
            && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &peel_sig, peel_pidx,
            )
            && !self.ir_sig_arg_expects_shared_borrow(&peel_sig, arg_index)
            && (crate::ir::signature_bridge::call_site_expects_owned_pass(&peel_sig, peel_pidx)
                || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &peel_sig, peel_pidx,
                ))
        {
            if coerced.starts_with("&mut ")
                || (coerced.starts_with('&') && !coerced.starts_with("&mut "))
            {
                coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                    .to_string();
            }
        }

        // Already-emitted `&T` formals must not grow a second `&` (LsmEngine::get → engine.get).
        // Also cover analyzer-inferred / Phase-2 `&str` formals (`identifier_already_ref`).
        if let Expression::Identifier { name, .. } = arg_expr {
            if (self.emitted_rust_ref_formals.contains(name)
                || self.binding_emits_as_rust_shared_ref(name)
                || self.identifier_already_ref(name)
                || self.str_ref_optimized_params.contains(name.as_str()))
                && !self.collection_key_owned_params.contains(name.as_str())
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && coerced != name.as_str()
            {
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced);
                if base == name.as_str() || base.starts_with(name.as_str()) {
                    coerced = base.to_string();
                }
            }
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
            let this_arg_expects_borrow = self.ir_sig_arg_expects_shared_borrow(&sig, arg_index);
            let this_arg_expects_mut = self.ir_sig_arg_expects_mut_borrow(&sig, arg_index);
            match arg_expr {
                Expression::Identifier { name, .. } => {
                    let arg_is_fn_param =
                        self.current_function_params.iter().any(|p| p.name == *name);
                    if self.borrowed_iterator_vars.contains(name)
                        && self.current_function_return_type.as_ref().is_some_and(|rt| {
                            matches!(rt, Type::Vec(inner) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_)))
                        })
                    {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut coerced,
                        );
                    } else if arg_is_fn_param
                        && (this_arg_expects_borrow || this_arg_expects_mut)
                    {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut coerced,
                        );
                    }
                }
                Expression::FieldAccess { object, .. } => {
                    // `&mut self.field` / `&self.field` — never `&mut self.field.clone()`.
                    if (this_arg_expects_borrow || this_arg_expects_mut)
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
                self.signature_registry.get_signature(callee_name).cloned()
                // Never fall back to the bare method name for `module::fn` /
                // `Type::method` — that matches unrelated homonyms
                // (e.g. rendering_api::draw_text for draw::draw_text).
            })
            .unwrap_or(sig.clone());
        let callee_pidx = callee_sig.arg_param_index(arg_index);
        let runtime_std_needs_borrow =
            crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                registry,
                callee_name,
                Some(&callee_sig),
                arg_index,
            ) || {
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
            CoercionKind::Identity
                | CoercionKind::Borrow
                | CoercionKind::MutBorrow
                | CoercionKind::ToOwnedString
                | CoercionKind::Clone
        )
            && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(&callee_sig, callee_pidx)
            && !runtime_std_needs_borrow
            && !(crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                && crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
                    Some(&callee_sig),
                    arg_index,
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
                coerced = coerced[1..].to_string();
            }
            if skip_clone_for_user_closure_param {
                if self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                    && !coerced.starts_with('&')
                    && !coerced.starts_with("&mut ")
                {
                    coerced = format!("&{coerced}");
                }
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
                let callee_accepts_ref_reborrow = callee_borrows_text
                    || callee_sig.param_types.get(callee_pidx).is_some_and(|t| {
                        matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    })
                    || self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                    || self.ir_sig_arg_expects_mut_borrow(&callee_sig, arg_index);
                if !formal_is_copy && !callee_accepts_ref_reborrow {
                    coerced = format!("{}.clone()", coerced.trim_start_matches('&'));
                }
            } else if !coerced.ends_with(".clone()") {
                let skip_user_closure_param = matches!(
                    arg_expr,
                    Expression::Identifier { name, .. }
                        if self.in_user_written_closure && self.user_closure_params.contains(name)
                );
                if skip_user_closure_param {
                    // Preserve user-written closure bodies (e.g. |e| predicate(e)).
                } else {
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
                    ) && !coerced.ends_with(".to_string()")
                        && !coerced.ends_with(".to_owned()")
                        && !match arg_expr {
                            // Only scalar Copy (i64/bool/…) skips clone; Copy aggregates/enums
                            // still need `.clone()` on multi-use owned moves (regression-063 Value).
                            Expression::Identifier { name, .. } => {
                                self.binding_is_copy_pass_by_value_scalar(name)
                            }
                            _ => self.infer_expression_type(arg_expr).is_some_and(|t| {
                                let bare = match &t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                crate::type_classification::is_copy_pass_by_value_formal(bare)
                            }),
                        }
                    {
                        coerced = format!("{}.clone()", coerced.trim_start_matches('&'));
                    }
                }
                }
                }
            }
            }
        }
        let _ = pidx;
        // Strip redundant `.clone()` only for scalar Copy formals (i64/bool/…).
        // Copy aggregates/enums (Value, Lsn) still need multi-use clones (regression-063).
        if coerced.ends_with(".clone()") {
            let formal_ty = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx));
            let formal_is_scalar_copy = formal_ty.is_some_and(|t| {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                crate::type_classification::is_copy_pass_by_value_formal(bare)
            });
            let binding_is_scalar_copy = match arg_expr {
                Expression::Identifier { name, .. } => {
                    self.binding_is_copy_pass_by_value_scalar(name)
                }
                _ => self.infer_expression_type(arg_expr).is_some_and(|t| {
                    let bare = match &t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    crate::type_classification::is_copy_pass_by_value_formal(bare)
                }),
            };
            if formal_is_scalar_copy || binding_is_scalar_copy {
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
            }
        }
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && !crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
            Some(&sig),
            arg_index,
        )
            && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                &sig, param_idx,
            )
            && !crate::ir::signature_bridge::call_site_expects_owned_pass(&sig, param_idx)
            && !crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &sig, arg_index,
            )
            && sig
            .param_type_for_arg(arg_index)
            .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_string_ref)
        {
            let base = coerced.trim_start_matches('&');
            let owned = if crate::codegen::rust::string_utilities::already_owned_string_expr(base)
            {
                base.to_string()
            } else {
                crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(base)
            };
            return Some(format!("&{owned}"));
        }
        // Plain WJ `string` formals emit owned `String` even when multipass left stale
        // `Borrowed` ownership (cross-file analysis before defining-module codegen).
        // Signature-driven only — no method-name heuristics.
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && !self.ir_sig_arg_expects_shared_borrow(
            sig_for_owned_literal.as_ref().unwrap_or(&sig),
            arg_index,
        ) && (crate::codegen::rust::string_utilities::string_literal_needs_to_string(
            sig_for_owned_literal.as_ref().unwrap_or(&sig),
            arg_index,
        ) || crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
            sig_for_owned_literal.as_ref().unwrap_or(&sig),
            arg_index,
        )) && !crate::codegen::rust::string_utilities::already_owned_string_expr(&coerced)
        {
            return Some(
                crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                    coerced.trim_start_matches('&'),
                ),
            );
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
            let fresh_sig = callee_name.rsplit_once("::").and_then(|(rt, method)| {
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
            let callee_wants_shared =
                crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    borrow_sig, borrow_idx,
                );
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
            && !sig
                .emitted_rust_ref_params
                .as_ref()
                .is_some_and(|flags| flags.get(param_idx).copied().unwrap_or(false))
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
            // Prefer codegen-refreshed registry entry — analyzer call-site stubs often
            // lack `emitted_rust_ref_params` while the defining-fn refresh has them
            // (`check(item: &Item)` after `fn check` body emission).
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            if let Some(refreshed) =
                crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    self.signature_registry.get_signature(callee_name).cloned(),
                    self.signature_registry.get_signature(simple).cloned(),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(callee_name).cloned()),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned()),
                    Some(sig.clone()),
                ])
            {
                let ridx = refreshed.arg_param_index(arg_index);
                if refreshed.emitted_rust_ref_params.is_some()
                    || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        &refreshed, ridx,
                    )
                {
                    sig = refreshed;
                    param_idx = ridx;
                }
            }
            let emits_shared =
                crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &sig, param_idx,
                );
            if std::env::var_os("WJ_DEBUG_COLLISION_BORROW").is_some() {
                eprintln!(
                    "WJ_DEBUG_COLLISION_BORROW callee={callee_name} arg={arg_index} \
                     param_idx={param_idx} emits_shared={emits_shared} \
                     emitted={:?} ownership={:?} param_ty={:?} coerced={coerced}",
                    sig.emitted_rust_ref_params,
                    sig.param_ownership.get(param_idx),
                    sig.param_types.get(param_idx),
                );
            }
            if !emits_shared {
                // Homonym collisions (`check`, `process`, …) strip unsafe auto-borrow
                // when the callee contract is ambiguous across modules.
                while coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                    coerced = coerced[1..].to_string();
                }
                if !matches!(arg_expr, Expression::Index { .. }) {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
                }
            } else {
                let is_text_shared = crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                    &sig, param_idx,
                ) || sig.param_types.get(param_idx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        || matches!(
                            t,
                            Type::Reference(inner)
                                if crate::codegen::rust::types::is_windjammer_text_type(inner)
                        )
                });
                if is_text_shared {
                    // Confirmed `&str`/`&String` under collision: keep `&`, drop owned
                    // literal coercion (string_literal_no_conversion / regression-048).
                    if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                        coerced = stripped.to_string();
                    }
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
                }
                // Confirmed shared-ref Custom (`item: &Item`): ensure `&` survives
                // collision even when earlier IR/should_borrow skipped the prefix.
                if !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                    && matches!(
                        arg_expr,
                        Expression::Identifier { .. } | Expression::FieldAccess { .. }
                    )
                {
                    let arg_already_rust_ref = matches!(
                        arg_expr,
                        Expression::Identifier { name, .. }
                            if self.identifier_binding_already_rust_ref(name)
                                || self.str_ref_optimized_params.contains(name.as_str())
                                || self.inferred_borrowed_params.contains(name)
                    );
                    if !arg_already_rust_ref {
                        // Drop owned-move artifacts introduced under stale collision
                        // stripping (`item.clone()` / `"lit".to_string()`).
                        if coerced.ends_with(".clone()") {
                            crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                &mut coerced,
                            );
                        }
                        if coerced.ends_with(".to_string()") && is_text_shared {
                            if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                                coerced = stripped.to_string();
                            }
                        }
                        if !coerced.starts_with('&') {
                            coerced = format!("&{coerced}");
                        }
                        if std::env::var_os("WJ_DEBUG_COLLISION_BORROW").is_some() {
                            eprintln!(
                                "WJ_DEBUG_COLLISION_BORROW after_force_ref coerced={coerced}"
                            );
                        }
                    }
                }
            }
        }
        // Final IR ownership contract: strip spurious `&` when callee emits owned formals
        // (regression-056 keys_equal(Vec<u8>, Vec<u8>)), or add borrow when expected.
        self.enforce_call_site_ownership_contract(
            &mut coerced,
            arg_expr,
            &sig,
            param_idx,
            callee_name,
            arg_index,
        );
        if std::env::var_os("WJ_DEBUG_COLLISION_BORROW").is_some() {
            eprintln!(
                "WJ_DEBUG_COLLISION_BORROW after_enforce callee={callee_name} coerced={coerced}"
            );
        }
        self.peel_copy_aggregate_caller_into_owned_callee(
            &mut coerced,
            arg_expr,
            &sig,
            arg_index,
            receiver_type_name,
        );
        if let Expression::Identifier { name, .. } = arg_expr {
            if !self.match_arm_bindings.contains(name.as_str())
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && self.caller_owned_non_copy_formal(name)
                && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                )
                && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    &self.signature_registry,
                    callee_name,
                    Some(&sig),
                    arg_index,
                )
            {
                coerced = coerced.trim_start_matches('&').to_string();
            }
        }
        // After borrow stripping / collision clone-stripping: restore `.clone()` when
        // auto-clone analysis says this binding/path is moved and reused (regression-059).
        coerced = self.ensure_owned_move_clone_for_reuse(arg_expr, &coerced, &sig, param_idx);
        crate::codegen::rust::expression_utilities::collapse_redundant_clones(&mut coerced);
        if coerced.ends_with(".to_string().clone()") || coerced.ends_with(".to_owned().clone()") {
            crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
        }
        if coerced.ends_with(".clone()") {
            if let Expression::Identifier { name, .. } = arg_expr {
                let collects_ref_vec =
                    crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                        self.current_function_return_type.as_ref(),
                    );
                if collects_ref_vec
                    && (self.borrowed_iterator_vars.contains(name)
                        || self.local_var_types.get(name).is_some_and(|t| {
                            matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        }))
                {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
                }
            }
        }
        // Match-arm owned String payloads: shared-ref text callees want `&binding`.
        // Owned WJ `string` formals must keep the move (no `&`).
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.match_arm_bindings.contains(name.as_str()) {
                let expects_shared_text = crate::ir::signature_bridge::call_site_expects_shared_borrow(
                    &sig, param_idx,
                ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &sig, param_idx,
                ) || sig.param_types.get(param_idx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                });
                if expects_shared_text {
                    if coerced.ends_with(".clone()") {
                        coerced = coerced[..coerced.len() - ".clone()".len()].to_string();
                    }
                    if !coerced.starts_with('&') {
                        coerced = format!("&{coerced}");
                    }
                }
            }
        }
        coerced =
            self.normalize_owned_copy_match_binding_call_arg(arg_expr, &coerced, &sig, arg_index);
        coerced = self.normalize_borrowed_iter_elem_for_owned_copy_scalar(
            arg_expr, &coerced, &sig, arg_index,
        );
        if coerced.ends_with(".to_string()")
            && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
        {
            if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                coerced = stripped.to_string();
            }
        }
        // `&str` / `&String` caller bindings → owned `String` formals need `.to_string()`
        // (types-crate `batch_column_i64(name: &str)` → `ArrowBatch::column_i64(name: String)`).
        if let Expression::Identifier { name, .. } = arg_expr {
            let caller_is_str_slice = self.str_ref_optimized_params.contains(name.as_str())
                || (self.emitted_rust_ref_formals.contains(name)
                    && self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    }));
            let callee_wants_owned_string =
                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                ) && sig.formal_param_type(param_idx).is_some_and(|t| {
                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        && crate::codegen::rust::types::is_windjammer_text_type(t)
                });
            if caller_is_str_slice
                && callee_wants_owned_string
                && !crate::codegen::rust::string_utilities::already_owned_string_expr(&coerced)
            {
                coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                    crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced),
                );
            }
        }
        // Recursive same-fn call into an owned formal emitted by *this* function:
        // strip stale `&` from analyzer/forward-ref borrow (ReBAC `policy: Policy`).
        // Must run last in apply_ir — reconcile also re-peels terminally.
        self.strip_recursive_owned_formal_stale_borrow(&mut coerced, arg_expr, callee_name);
        let expects_str_ref = crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
            &sig, arg_index,
        ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
            &sig, param_idx,
        ) || receiver_type_name.is_some_and(|rt| {
            crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
                method_simple,
                Some(rt),
                registry,
                arg_index,
            )
        });
        if expects_str_ref {
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                &mut coerced,
            );
        }
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && coerced.starts_with('&')
            && coerced.ends_with(".to_string()")
            && (crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &sig, arg_index,
            ) || crate::ir::signature_bridge::call_site_expects_owned_pass(&sig, param_idx))
        {
            coerced = coerced.trim_start_matches('&').to_string();
        }

        // Single IR finalize pass: collection keys + string literals (signature-driven).
        // Callers must not re-implement collection-key prefix/strip after apply_ir returns.
        self.finalize_ir_collection_key_arg(
            &mut coerced,
            arg_expr,
            &sig,
            arg_index,
            receiver_type_name,
        );
        crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
            Some(&sig),
            arg_index,
            Some(method_simple),
            arg_expr,
            &mut coerced,
            receiver_type_name,
            Some(&self.enum_variant_types),
        );
        if crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
            &sig,
            arg_index,
            receiver_type_name,
        ) && matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) {
            if coerced.ends_with(".to_string()") {
                if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                    coerced = stripped.to_string();
                }
            }
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                &mut coerced,
            );
        }

        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && crate::codegen::rust::string_utilities::already_owned_string_expr(&coerced)
            && !crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
                &sig,
                arg_index,
                receiver_type_name,
            )
        {
            coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(&coerced);
        }

        if self.in_if_condition {
            if let Expression::Identifier { name, .. } = arg_expr {
                if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                ) && self.current_function_params.iter().any(|p| {
                    p.name == *name && !self.is_type_copy(&p.type_)
                })
                    && !coerced.ends_with(".clone()")
                {
                    coerced = format!("{coerced}.clone()");
                }
            }
        }

        Some(coerced)
    }

    /// Signature-driven map/set key finalize: strip `&&` on already-shared bindings,
    /// then ensure `&K` when the formal is a collection-key lookup.
    ///
    /// Binding awareness lives here (IR) so method/free-call sites stay DRY.
    fn finalize_ir_collection_key_arg(
        &self,
        arg_str: &mut String,
        arg_expr: &Expression<'ast>,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
        receiver_type_name: Option<&str>,
    ) {
        if !crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
            sig,
            arg_index,
            receiver_type_name,
        ) {
            return;
        }

        let binding_name =
            crate::codegen::rust::call_site_borrow::borrow_target_identifier_name(arg_expr);
        let binding_already_shared = binding_name.as_ref().is_some_and(|name| {
            self.emitted_rust_ref_formals.contains(name)
                || self.str_ref_optimized_params.contains(name.as_str())
                || self.binding_emits_as_rust_shared_ref(name)
                || self.identifier_already_ref(name)
                || (self.inferred_borrowed_params.contains(name.as_str())
                    && !self.collection_key_owned_params.contains(name.as_str()))
        });
        let text_param_already_shared = binding_name.as_ref().is_some_and(|name| {
            !self.collection_key_owned_params.contains(name.as_str())
                && self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                })
                && (self.emitted_rust_ref_formals.contains(name)
                    || self.str_ref_optimized_params.contains(name.as_str())
                    || self.inferred_borrowed_params.contains(name.as_str())
                    || self.identifier_already_ref(name))
        });
        let arg_already_rust_ref = binding_name.as_ref().is_some_and(|name| {
            self.identifier_already_ref(name)
                || self.emitted_rust_ref_formals.contains(name)
                || self.str_ref_optimized_params.contains(name.as_str())
                || self.inferred_borrowed_params.contains(name.as_str())
        });

        // Drop a spurious leading `&` when the binding already lowers as `&T` / `&str`
        // (HashMap::get(key) never HashMap::get(&key) → &&str).
        if binding_already_shared
            || text_param_already_shared
            || binding_name
                .as_ref()
                .is_some_and(|name| self.current_function_params.iter().any(|p| p.name == *name))
        {
            crate::codegen::rust::call_site_borrow::strip_redundant_borrow_on_ref_binding(
                arg_expr, arg_str,
            );
        }

        crate::codegen::rust::call_site_borrow::finalize_collection_key_call_site_arg(
            Some(sig),
            arg_index,
            arg_expr,
            arg_str,
            arg_already_rust_ref,
            receiver_type_name,
            binding_already_shared || text_param_already_shared,
        );
    }

    /// When a binding/path is moved and reused, and the final argument is passed by
    /// value (no leading `&`), ensure `.clone()` is present (regression-059).
    fn ensure_owned_move_clone_for_reuse(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        sig: &crate::analyzer::FunctionSignature,
        param_idx: usize,
    ) -> String {
        if let Some(rewritten) = self.try_self_field_writeback_owned_arg(arg_expr, arg_str) {
            return rewritten;
        }
        if arg_str.ends_with(".clone()") || arg_str.starts_with('*') {
            return arg_str.to_string();
        }
        if arg_str.contains("std::mem::take(") {
            return arg_str.to_string();
        }
        if arg_str.ends_with(".to_string()") {
            return arg_str.to_string();
        }
        // `if !callee(mut_param)` then `{ mut_param.mutate() }` — owned formal in the
        // condition moves before the then-branch reuses the binding (list_unique / ReBAC).
        if self.in_if_condition {
            if let Expression::Identifier { name, .. } = arg_expr {
                if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                ) && self.current_function_params.iter().any(|p| {
                    p.name == *name && !self.is_type_copy(&p.type_)
                }) {
                    return format!("{arg_str}.clone()");
                }
            }
        }
        // Indexing a non-Copy element into a non-shared-ref formal is always an
        // invalid move (E0507). Analyzer may still mark `(Row, T)` chain helpers
        // Borrowed while codegen emits owned `Row` — trust shared-ref emission.
        if matches!(arg_expr, Expression::Index { .. }) {
            let emits_shared = [sig.name.as_str(), sig.name.rsplit("::").next().unwrap_or(&sig.name)]
                .iter()
                .find_map(|key| {
                    self.signature_registry
                        .get_signature(key)
                        .or_else(|| {
                            self.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(key))
                        })
                        .and_then(|s| s.emitted_rust_ref_params.as_ref())
                        .and_then(|flags| flags.get(param_idx).copied())
                })
                .unwrap_or_else(|| {
                    sig.emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(param_idx).copied())
                        .unwrap_or_else(|| {
                            crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                sig, param_idx,
                            )
                        })
                });
            if !emits_shared {
                let needs_clone = match self.infer_expression_type(arg_expr) {
                    None => true,
                    Some(t) => {
                        let bare = match &t {
                            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                            other => other,
                        };
                        !self.is_type_copy(bare) || matches!(bare, Type::Custom(_))
                    }
                };
                if needs_clone {
                    return format!("{arg_str}.clone()");
                }
            }
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.match_arm_bindings.contains(name.as_str()) {
                let mut out = arg_str.to_string();
                if out.ends_with(".clone()") {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut out);
                }
                let expects_shared_text = crate::ir::signature_bridge::call_site_expects_shared_borrow(
                    sig, param_idx,
                ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    sig, param_idx,
                ) || sig.param_types.get(param_idx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                });
                if expects_shared_text && !out.starts_with('&') {
                    out = format!("&{out}");
                }
                return out;
            }
            if (self.borrowed_iterator_vars.contains(name)
                || self
                    .local_var_types
                    .get(name)
                    .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_))))
                && self
                    .current_function_return_type
                    .as_ref()
                    .is_some_and(|rt| {
                        crate::codegen::rust::types::return_type_is_vec_of_shared_refs(Some(rt))
                    })
            {
                let mut out = arg_str.to_string();
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut out);
                return out;
            }
        }
        // Mut-ref formals must keep `&mut binding` — never append `.clone()`.
        if arg_str.starts_with("&mut ") {
            return arg_str.to_string();
        }
        // When reuse requires clone but a stale shared-borrow prefix was applied
        // (`&value` into owned `value: Value`), strip `&` and clone (regression-063).
        if arg_str.starts_with('&') {
            let Some(ref analysis) = self.auto_clone_analysis else {
                return arg_str.to_string();
            };
            // Statement-local reuse only — `needs_clone_anywhere` falsely clones
            // discard-only / single-use owned args reused across calls (authz-reuse regression).
            let needs = match arg_expr {
                Expression::Identifier { name, .. } => analysis
                    .needs_clone(name, self.current_statement_idx)
                    .is_some(),
                Expression::FieldAccess { .. } | Expression::Index { .. } => {
                    Self::auto_clone_expr_path(arg_expr).is_some_and(|path| {
                        analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                    })
                }
                _ => false,
            };
            if needs
                && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                )
            {
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str);
                let skip = match arg_expr {
                    Expression::Identifier { name, .. } => {
                        self.binding_is_copy_pass_by_value_scalar(name)
                    }
                    _ => false,
                };
                if !skip {
                    return format!("{base}.clone()");
                }
                return base.to_string();
            }
            return arg_str.to_string();
        }
        if matches!(
            sig.param_ownership.get(param_idx),
            Some(crate::analyzer::OwnershipMode::MutBorrowed)
        ) || sig
            .param_types
            .get(param_idx)
            .is_some_and(|t| matches!(t, Type::MutableReference(_)))
        {
            return arg_str.to_string();
        }
        let Some(ref analysis) = self.auto_clone_analysis else {
            return arg_str.to_string();
        };
        // Statement-local reuse only (matches method-arg policy in arguments.rs).
        let needs = match arg_expr {
            Expression::Identifier { name, .. } => {
                let local = analysis
                    .needs_clone(name, self.current_statement_idx)
                    .is_some();
                let anywhere = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                ) && !self.binding_is_copy_pass_by_value_scalar(name)
                    && analysis.needs_clone_anywhere(name);
                local || anywhere
            }
            Expression::FieldAccess { .. } | Expression::Index { .. } => {
                Self::auto_clone_expr_path(arg_expr).is_some_and(|path| {
                    analysis
                        .needs_clone(&path, self.current_statement_idx)
                        .is_some()
                })
            }
            _ => false,
        };
        if needs {
            if crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, param_idx) {
                if arg_str.starts_with('&') {
                    return arg_str.to_string();
                }
                return format!(
                    "&{}",
                    crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str)
                );
            }
            // Scalar Copy formals (i64/bool/…) need no clone; Copy aggregates/enums still do.
            let skip_clone = match arg_expr {
                Expression::Identifier { name, .. } => {
                    self.binding_is_copy_pass_by_value_scalar(name)
                        || self
                            .current_function_params
                            .iter()
                            .find(|p| p.name == *name)
                            .is_some_and(|p| self.is_type_copy(&p.type_))
                        || self
                            .local_var_types
                            .get(name)
                            .is_some_and(|t| self.is_type_copy(t))
                }
                _ => self.infer_expression_type(arg_expr).is_some_and(|t| {
                    let bare = match &t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    self.is_type_copy(bare)
                }),
            };
            if skip_clone {
                arg_str.to_string()
            } else {
                format!("{arg_str}.clone()")
            }
        } else {
            arg_str.to_string()
        }
    }

    /// Enforce the ownership contract for a single call-site argument by computing
    /// actual vs expected safety types and applying the coercion (strip `&`, add `.clone()`, etc.).
    /// Signature-driven: explicit `&x` at call site → owned callee formal gets `*x` (Copy) or `x.clone()`.
    pub(crate) fn coerce_explicit_ref_for_owned_callee_arg(
        &self,
        arg_expr: &Expression<'ast>,
        mut arg_str: String,
        sig: Option<&crate::analyzer::FunctionSignature>,
        arg_index: usize,
    ) -> String {
        if !crate::codegen::rust::expression_helpers::is_reference_expression(arg_expr) {
            return arg_str;
        }
        let Some(sig) = sig else {
            return arg_str;
        };
        let callee_wants_borrow =
            sig.param_types
                .get(arg_index)
                .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)))
                || matches!(
            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                sig, arg_index,
            ),
            crate::analyzer::OwnershipMode::Borrowed | crate::analyzer::OwnershipMode::MutBorrowed,
        );
        if callee_wants_borrow {
            return arg_str;
        }
        let formal_idx = sig.arg_param_index(arg_index);
        let callee_formal_is_copy = sig
            .formal_param_type(formal_idx)
            .or_else(|| sig.param_types.get(formal_idx))
            .is_some_and(|t| match t {
                Type::Reference(inner) | Type::MutableReference(inner) => self.is_type_copy(inner),
                other => self.is_type_copy(other),
            });
        // Undo erroneous `*(&x)` / `*&x` from legacy deref coercion on explicit borrows.
        while arg_str.starts_with('*') {
            arg_str = arg_str[1..].to_string();
        }
        if arg_str.ends_with(".clone()") {
            return arg_str;
        }
        if callee_formal_is_copy {
            // Explicit `&owned` / `(&owned)` → owned Copy formal: pass by value
            // (Rust auto-copies). Never emit `*owned` — that E0614s when `owned`
            // is already a value local (IR may have stripped `&` already).
            let mut s = arg_str.trim().to_string();
            if s.starts_with('(') && s.ends_with(')') {
                let inner = s[1..s.len() - 1].trim().to_string();
                if inner.starts_with('&') || inner.starts_with("&mut ") {
                    s = inner;
                }
            }
            return crate::codegen::rust::expression_utilities::borrow_base_expr(&s).to_string();
        }
        let inner = crate::codegen::rust::expression_utilities::borrow_base_expr(&arg_str);
        // Parenthesized unary refs from expression codegen: `(&x)`.
        let inner = if arg_str.trim().starts_with('(') && arg_str.trim().ends_with(')') {
            let peeled = arg_str.trim();
            let mid = peeled[1..peeled.len() - 1].trim();
            if mid.starts_with('&') || mid.starts_with("&mut ") {
                crate::codegen::rust::expression_utilities::borrow_base_expr(mid)
            } else {
                inner
            }
        } else {
            inner
        };
        format!("{inner}.clone()")
    }

    /// Post-IR reconcile: apply mut-borrow and peel stale `&`/`&mut` for owned formals.
    ///
    /// Single source of truth for the clusters previously duplicated in
    /// `regular_call_arguments` / method `arguments` after `apply_ir_call_site_coercion`.
    /// Signature-driven only — no method-name ownership heuristics.
    pub(crate) fn reconcile_post_ir_mut_borrow_and_owned_peel(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
        registry: &SignatureRegistry,
        receiver_type_name: Option<&str>,
        receiver: Option<&Expression<'ast>>,
        user_arg_count: Option<usize>,
        has_ownership_collision: bool,
    ) {
        // Prefer defining-module codegen refresh (`emitted_rust_ref_params`) over stale
        // importer/collision stubs before any owned-formal peel (WDB-101 map getters).
        let sig = self.refreshed_call_site_sig_for_arg(registry, callee_name, arg_index, sig);
        let param_idx = sig.arg_param_index(arg_index);
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let global_confirms_shared_ref = |pidx: usize| {
            self.global_signature_registry.as_ref().is_some_and(|g| {
                [callee_name, simple]
                    .into_iter()
                    .flat_map(|key| {
                        [
                            g.get_signature(key),
                            g.lookup_method(key),
                            g.find_unique_signature_ending_with(simple),
                        ]
                    })
                    .flatten()
                    .any(|gs| {
                        crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            gs, pidx,
                        )
                    })
            })
        };

        // Terminal peel first: IR / collision paths may prefix `&` before reconcile runs.
        self.peel_stacked_amp_on_emitted_ref_binding(
            coerced,
            arg_expr,
            Some(&sig),
            arg_index,
            false,
        );

        // Ownership-collision: do not keep IR/heuristic `&` from a conflicting
        // Borrowed snapshot (draw_text homonyms). Confirmed shared-ref formals skip.
        if has_ownership_collision
            && crate::codegen::rust::call_signature_resolution::ownership_collision_blocks_autoborrow(
                simple,
            )
            && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            && !global_confirms_shared_ref(param_idx)
        {
            crate::codegen::rust::call_signature_resolution::strip_collision_blocked_call_site_coercions(
                coerced,
            );
        }

        // Prefer-shared enforce without stripping IR-confirmed shared refs (bug_e0308).
        self.enforce_ir_ownership_preserving_confirmed_shared_ref(
            coerced,
            arg_expr,
            callee_name,
            arg_index,
            &sig,
            registry,
        );
        if std::env::var_os("WJ_DEBUG_COLLISION_BORROW").is_some()
            && callee_name.contains("graph_vertex_i64_get")
            && arg_index == 0
        {
            eprintln!("WJ_DEBUG_COLLISION_BORROW after_enforce_ir coerced={coerced}");
        }

        // Spurious `&mut` on owned Copy-aggregate formals (regression-060).
        self.peel_spurious_mut_borrow_on_owned_copy_aggregate(
            coerced,
            callee_name,
            arg_index,
            &sig,
        );

        // Stale `&` on owned user free-fn formals (circular-dep / multipass).
        let skip_stale_borrow = !callee_name.contains("::")
            && crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn_with_global(
                registry,
                self.global_signature_registry.as_deref(),
                callee_name,
                &sig,
                param_idx,
                arg_index,
            );
        if skip_stale_borrow
            && (coerced.starts_with("&mut ")
                || (coerced.starts_with('&') && !coerced.starts_with("&mut ")))
            && !self.ir_callee_arg_expects_mut_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                Some(&sig),
            )
        {
            *coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        }
        if std::env::var_os("WJ_DEBUG_COLLISION_BORROW").is_some()
            && callee_name.contains("graph_vertex_i64_get")
            && arg_index == 0
        {
            eprintln!(
                "WJ_DEBUG_COLLISION_BORROW after_skip_stale skip={skip_stale_borrow} coerced={coerced}"
            );
        }

        // Shared-borrow reapply when IR used a stale stub and the refreshed sig borrows.
        // Skip on ownership collision unless codegen confirmed shared-ref emission.
        let allow_shared = !has_ownership_collision
            || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            || global_confirms_shared_ref(param_idx);
        if allow_shared
            && !skip_stale_borrow
            && !crate::codegen::rust::expression_helpers::is_reference_expression(arg_expr)
        {
            let skip_recursive_owned = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.current_function_name.as_deref().is_some_and(|cur| {
                        callee_name == cur
                            || callee_name.rsplit("::").next().is_some_and(|s| s == cur)
                    }) && self.current_function_params.iter().any(|p| p.name == *name)
                        && !self.emitted_rust_ref_formals.contains(name)
                        && !self.str_ref_optimized_params.contains(name.as_str())
            );
            if !skip_recursive_owned {
                let arg_already_rust_ref = matches!(
                    arg_expr,
                    Expression::Identifier { name, .. }
                        if self.identifier_binding_already_rust_ref(name)
                );
                if arg_already_rust_ref {
                    // Binding is already `&T` / `&mut T` in Rust — never prefix another `&`
                    // (`take_in_edges(&csr)` → `&&mut DenseCsr`).
                    *coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(
                        coerced,
                    )
                    .to_string();
                    // Keep `.clone()` when the slot is owned (iterator `push(item)` into
                    // `Vec<T>`). Only strip clone for true reborrows into `&` / `&mut`.
                    let wants_owned = crate::ir::signature_bridge::call_site_expects_owned_pass(
                        &sig, param_idx,
                    ) || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, param_idx,
                    );
                    if !wants_owned {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
                    }
                } else {
                let method = callee_name.rsplit("::").next().unwrap_or(callee_name);
                let formal_is_copy = sig
                    .formal_param_type(param_idx)
                    .or_else(|| sig.param_types.get(param_idx))
                    .or_else(|| sig.param_type_for_arg(arg_index))
                    .is_some_and(|t| {
                        let bare = match t {
                            Type::Reference(inner) | Type::MutableReference(inner) => {
                                inner.as_ref()
                            }
                            other => other,
                        };
                        crate::codegen::rust::call_site_borrow::bare_type_is_copy_aggregate_owned_formal(
                            bare,
                            |ty| self.is_type_copy(ty),
                        )
                    });
                let decision =
                    crate::codegen::rust::call_site_borrow::should_borrow_at_call_site_with_copy_check(
                        &sig,
                        arg_index,
                        arg_expr,
                        coerced,
                        method,
                        arg_already_rust_ref,
                        None,
                        formal_is_copy,
                    );
                crate::codegen::rust::call_site_borrow::apply_call_site_borrow(&decision, coerced);
                }
            }
        }

        // Mut-borrow from signature / codegen-recorded mut slots.
        let wants_mut = self.ir_callee_arg_expects_mut_borrow(
            registry,
            callee_name,
            arg_index,
            user_arg_count,
            Some(&sig),
        ) || self.ir_sig_arg_expects_mut_borrow(&sig, arg_index);
        if (!has_ownership_collision || wants_mut)
            && wants_mut
            && !coerced.starts_with("&mut ")
            && crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(
                arg_expr,
            )
        {
            // Already-emitted `&mut T` / `&T` bindings reborrow by value of the binding.
            let already_ref_binding = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.identifier_binding_already_rust_ref(name)
            );
            if already_ref_binding {
                *coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                    .to_string();
                // Mut reborrow never needs `.clone()` on an already-`&mut` binding.
                crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
            } else {
                crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                    arg_expr,
                    coerced,
                    &self.current_function_params,
                    &self.inferred_mut_borrowed_params,
                );
            }
        }

        // Owned formals: strip stale `&` / `&mut`. Prefer *any* layered signature
        // candidate that confirms owned emission (defining-module refresh beats a
        // stale Borrowed stub — dogfood `policy: Policy`).
        self.peel_stale_borrow_for_multi_candidate_owned_formal(
            coerced,
            callee_name,
            arg_index,
            &sig,
            registry,
            wants_mut,
        );
        if std::env::var_os("WJ_DEBUG_COLLISION_BORROW").is_some()
            && callee_name.contains("graph_vertex_i64_get")
            && arg_index == 0
        {
            eprintln!("WJ_DEBUG_COLLISION_BORROW after_multi_peel coerced={coerced}");
        }

        // Recursive same-fn call into owned formal emitted by this function.
        self.strip_recursive_owned_formal_stale_borrow(coerced, arg_expr, callee_name);

        // Prefer defining-module shared-text formals (`path: &str`) over stale owned stubs
        // before the IR text/collection finalize pass (regression-049).
        // Type-qualified callees never consult bare method homonyms (`log::error`).
        let mut text_sig_candidates: Vec<Option<crate::analyzer::FunctionSignature>> = self
            .global_signature_registry
            .as_ref()
            .map(|g| {
                crate::codegen::rust::signature_promotion::callee_signature_lookup_candidates(
                    g,
                    callee_name,
                )
            })
            .unwrap_or_default()
            .into_iter()
            .map(Some)
            .collect();
        text_sig_candidates.extend(
            crate::codegen::rust::signature_promotion::callee_signature_lookup_candidates(
                registry,
                callee_name,
            )
            .into_iter()
            .map(Some),
        );
        text_sig_candidates.push(Some(sig.clone()));
        let mut text_sig = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature(
            text_sig_candidates,
        )
        .unwrap_or_else(|| sig.clone());
        let pidx_for_upgrade = text_sig.arg_param_index(arg_index);
        let type_qualified =
            crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                callee_name,
            );
        let text_challengers: Vec<Option<&crate::analyzer::FunctionSignature>> = if type_qualified {
            vec![
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(callee_name)),
                registry.get_signature(callee_name),
            ]
        } else {
            vec![
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(callee_name)),
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(simple)),
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.find_unique_signature_ending_with(simple)),
                registry.get_signature(callee_name),
                registry.get_signature(simple),
                registry.find_unique_signature_ending_with(simple),
            ]
        };
        for challenger in text_challengers {
            text_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                Some(text_sig),
                challenger,
                pidx_for_upgrade,
            )
            .unwrap_or_else(|| sig.clone());
        }

        let arg_already_rust_ref = matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.identifier_binding_already_rust_ref(name)
                    || self.str_ref_optimized_params.contains(name.as_str())
                    || self.inferred_borrowed_params.contains(name)
        );
        let skip_borrow_finalize = matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && (coerced.ends_with(".to_string()")
            || coerced.ends_with(".to_owned()"))
            && {
                let pidx = sig.arg_param_index(arg_index);
                crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                    &sig, pidx,
                ) && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &sig, pidx,
                )
            };
        if !skip_borrow_finalize
            && !(has_ownership_collision
                && crate::codegen::rust::call_signature_resolution::ownership_collision_blocks_autoborrow(
                    callee_name,
                )
                && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &sig,
                    sig.arg_param_index(arg_index),
                )
                && matches!(
                    arg_expr,
                    Expression::Identifier { .. } | Expression::Literal { .. }
                ))
        {
            // FieldAccess still finalizes under collision (regression-049 confirmed `&str`).
            crate::codegen::rust::string_utilities::finalize_borrowed_text_call_site_arg(
                Some(&text_sig),
                arg_index,
                receiver_type_name,
                arg_expr,
                coerced,
                arg_already_rust_ref,
            );
        }
        if matches!(
            arg_expr,
            Expression::FieldAccess { .. } | Expression::Index { .. }
        ) {
            let pidx = text_sig.arg_param_index(arg_index);
            *coerced = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                &Some(text_sig.clone()),
                pidx,
                arg_expr,
                coerced.clone(),
                false,
            );
            // Owned plain WJ `string` formals (trait `authenticate(email: string)`) must
            // receive field moves/clones — never force `&request.email` from stale
            // body-converged `&str` emission on the impl.
            let owned_plain_string =
                crate::codegen::rust::call_site_borrow::plain_string_formal_passes_owned_at_call_site(
                    &text_sig, pidx,
                ) || crate::ir::signature_bridge::call_site_expects_owned_pass(&text_sig, pidx)
                    || (matches!(
                        text_sig.param_ownership.get(pidx),
                        Some(crate::analyzer::OwnershipMode::Owned)
                    ) && crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                        &text_sig, pidx,
                    ))
                    || self.global_signature_registry.as_ref().is_some_and(|g| {
                        crate::codegen::rust::call_signature_resolution::global_trait_owned_plain_string_arg(
                            g, simple, arg_index,
                        )
                    });
            if owned_plain_string && coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                *coerced = crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                    coerced,
                );
            }
            if !owned_plain_string
                && !coerced.starts_with('&')
                && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &text_sig, pidx,
                )
                && (crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                    &text_sig, pidx,
                ) || text_sig.param_types.get(pidx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        || matches!(
                            t,
                            Type::Reference(inner)
                                if crate::codegen::rust::types::is_windjammer_text_type(inner)
                        )
                }))
                && self
                    .infer_expression_type(arg_expr)
                    .as_ref()
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
            {
                *coerced = format!("&{coerced}");
            }
        }

        // Re-apply IR collection-key / string-literal finalize after owned peels so
        // HashMap::get / Set::contains keep `&K` (and strip `&&` on shared bindings).
        self.finalize_ir_collection_key_arg(
            coerced,
            arg_expr,
            &text_sig,
            arg_index,
            receiver_type_name,
        );
        crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
            Some(&text_sig),
            arg_index,
            Some(simple),
            arg_expr,
            coerced,
            receiver_type_name,
            Some(&self.enum_variant_types),
        );
        if crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
            &text_sig,
            arg_index,
            receiver_type_name,
        ) && matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) {
            if coerced.ends_with(".to_string()") {
                if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                    *coerced = stripped.to_string();
                }
            }
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                coerced,
            );
        }

        // Vec locals into `&Vec<T>` formals (signature-driven; not in apply_ir).
        *coerced =
            crate::codegen::rust::call_site_borrow::maybe_borrow_owned_vec_local_for_ref_formal(
                self,
                &text_sig,
                arg_index,
                arg_expr,
                std::mem::take(coerced),
                receiver_type_name,
                Some(simple),
                user_arg_count,
            );
        // Owned Vec/non-Copy formals: stale `&binding` from reuse must become `.clone()`
        // (ReBAC `contains_string(&out)` into `items: Vec<String>`).
        {
            let owned_pidx = text_sig.arg_param_index(arg_index);
            let callee_emits_shared =
                crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &text_sig, owned_pidx,
                ) || global_confirms_shared_ref(owned_pidx);
            let bare_is_vec = text_sig
                .formal_param_type(owned_pidx)
                .or_else(|| text_sig.param_types.get(owned_pidx))
                .is_some_and(|t| {
                    let bare = match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    matches!(bare, Type::Vec(_))
                        || matches!(bare, Type::Parameterized(n, _) if n == "Vec")
                });
            let analyzer_borrows_vec = matches!(
                text_sig.param_ownership.get(owned_pidx),
                Some(
                    crate::analyzer::OwnershipMode::Borrowed
                        | crate::analyzer::OwnershipMode::MutBorrowed
                )
            );
            let owned_slot = !callee_emits_shared
                && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                &text_sig, owned_pidx,
            ) || (crate::ir::signature_bridge::call_site_expects_owned_pass(
                &text_sig, owned_pidx,
            )
                && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &text_sig, owned_pidx,
                ))
                || (bare_is_vec
                && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &text_sig, owned_pidx,
                )
                // Analyzer-Borrowed / IR shared-ref Vec formals keep `&walls`
                // (cross-file `check_collisions`); bare WJ `Vec` alone is not owned.
                && !analyzer_borrows_vec
                && !crate::ir::signature_bridge::call_site_expects_shared_borrow(
                    &text_sig, owned_pidx,
                )
                && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &text_sig, owned_pidx,
                ) || matches!(
                    text_sig.param_ownership.get(owned_pidx),
                    Some(crate::analyzer::OwnershipMode::Owned)
                ) || text_sig.formal_param_type(owned_pidx).is_some_and(|t| {
                    matches!(t, Type::Vec(_))
                        || matches!(t, Type::Parameterized(n, _) if n == "Vec")
                }))));
            if coerced.starts_with('&') && !coerced.starts_with("&mut ") && owned_slot {
                if std::env::var_os("WJ_DEBUG_COLLISION_BORROW").is_some()
                    && callee_name.contains("graph_vertex_i64_get")
                    && arg_index == 0
                {
                    eprintln!(
                        "WJ_DEBUG_COLLISION_BORROW owned_slot_peel callee={callee_name} \
                         callee_emits_shared={callee_emits_shared} owned_slot={owned_slot}"
                    );
                }
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                    .to_string();
                let needs_clone = match arg_expr {
                    Expression::Identifier { name, .. } => {
                        self.auto_clone_analysis.as_ref().is_some_and(|a| {
                            a.needs_clone(name, self.current_statement_idx).is_some()
                                || a.needs_clone_anywhere(name)
                        })
                    }
                    _ => false,
                };
                *coerced = if needs_clone
                    && !matches!(
                        arg_expr,
                        Expression::Identifier { name, .. }
                            if self.binding_is_copy_pass_by_value_scalar(name)
                    ) {
                    format!("{base}.clone()")
                } else {
                    // `&Vec` into an owned/`Vec` formal is never valid; clone mut locals.
                    match arg_expr {
                        Expression::Identifier { name, .. }
                            if self.local_var_types.get(name).is_some_and(|t| {
                                matches!(t, Type::Vec(_))
                                    || matches!(t, Type::Parameterized(n, _) if n == "Vec")
                            }) =>
                        {
                            format!("{base}.clone()")
                        }
                        _ => base,
                    }
                };
            }
        }

        // Terminal: owned `string` formals must not keep `&"lit".to_string()`.
        let text_param_idx = text_sig.arg_param_index(arg_index);
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && coerced.starts_with('&')
            && coerced.ends_with(".to_string()")
            && (crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                callee_name,
            ) || (crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                &text_sig, text_param_idx,
            ) && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &text_sig, text_param_idx,
            )) || crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &text_sig, arg_index,
            ) || crate::ir::signature_bridge::call_site_expects_owned_pass(
                &text_sig, text_param_idx,
            ))
        {
            *coerced = coerced.trim_start_matches('&').to_string();
        }

        if coerced.ends_with(".to_string().clone()") || coerced.ends_with(".to_owned().clone()") {
            crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
        }

        // Pattern / `&str` formals: `"lit".to_string()` / `String::from("lit")` → `"lit"`.
        let expects_str_ref = crate::codegen::rust::string_utilities::method_call_arg_expects_pattern_str(
            simple,
            arg_index,
            Some(&text_sig),
            receiver_type_name,
            receiver_type_name.is_some_and(|rt| {
                crate::codegen::rust::types::is_windjammer_text_type(&Type::Custom(rt.to_string()))
                    || rt == "str"
                    || rt.ends_with("::String")
                    || rt.ends_with("::str")
            }),
            registry,
        ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
            &text_sig, text_param_idx,
        ) || text_sig
            .param_types
            .get(text_param_idx)
            .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref)
            // Methods with Borrowed WJ `string` formals lower to Rust `&str`.
            || (text_sig.has_self_receiver_slot()
                && crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                    &text_sig, text_param_idx,
                )
                && matches!(
                    text_sig.param_ownership.get(text_param_idx),
                    Some(crate::analyzer::OwnershipMode::Borrowed)
                ));
        if expects_str_ref {
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                coerced,
            );
        }

        self.ensure_shared_borrow_on_match_arm_readonly_text(
            coerced, arg_expr, &text_sig, arg_index,
        );

        // Runtime-std WJ-owned / Rust-borrowed slots (`json::get` `&Value`) — signature
        // registry, never module-name lists.
        if crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
            registry,
            callee_name,
            Some(&text_sig),
            arg_index,
        ) && !coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
            && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
            && matches!(
                arg_expr,
                Expression::Identifier { .. } | Expression::FieldAccess { .. }
            )
            && !matches!(
                arg_expr,
                Expression::Identifier { name, .. } if self.identifier_already_ref(name)
            )
        {
            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(coerced);
        }

        // Empty/stub WJ sigs: type-qualified associated + unresolved instance builders
        // still auto-own bare string lits (signature-driven; no `new`/`from` name lists).
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && (crate::codegen::rust::string_utilities::type_qualified_associated_string_literal_needs_rust_owned_string(
            callee_name,
            arg_index,
            Some(&text_sig),
            registry,
            self.global_signature_registry.as_deref(),
        ) || crate::codegen::rust::string_utilities::unresolved_instance_method_string_literal_needs_rust_owned_string(
            simple,
            arg_index,
            Some(&text_sig),
            registry,
            self.global_signature_registry.as_deref(),
            receiver_type_name,
        )) && !crate::codegen::rust::string_utilities::already_owned_string_expr(coerced)
        {
            *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                coerced.trim_start_matches('&'),
            );
        }

        self.strip_stale_amp_on_already_ref_arg(arg_expr, coerced);

        // Mixed-forwarder / owned-outer / reuse-clone / pure-forwarding: IR coercion
        // for self-receiver calls, then shared helpers. Copy-aggregate peel runs after
        // so `&through` into owned `Lsn` is still stripped (regression-060).
        self.apply_post_ir_forwarder_owned_outer_and_reuse(
            coerced, arg_expr, &sig, arg_index, receiver,
        );

        // After shared-borrow reapply: Copy-aggregate caller → owned Copy-aggregate
        // callee must not keep stale `&` (regression-060).
        self.peel_copy_aggregate_caller_into_owned_callee(
            coerced,
            arg_expr,
            &text_sig,
            arg_index,
            receiver_type_name,
        );

        // Signature-driven numeric formals (usize index/capacity, int→float).
        // Use the call-site contract `sig`, not `text_sig` — suffix refresh of
        // `insert` can pick Vec::insert (usize) over HashMap::insert (K, V).
        self.apply_post_ir_numeric_formal_casts(
            coerced,
            arg_expr,
            callee_name,
            arg_index,
            &sig,
            receiver_type_name,
        );

        // Terminal: strip collision-blocked borrows again after text/forwarder
        // finalize may have re-applied `&` from a conflicting Borrowed snapshot.
        let collision_simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        if has_ownership_collision
            && crate::codegen::rust::call_signature_resolution::ownership_collision_blocks_autoborrow(
                collision_simple,
            )
            && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            && !global_confirms_shared_ref(param_idx)
            && matches!(
                arg_expr,
                Expression::Identifier { .. } | Expression::Literal { .. }
            )
        {
            crate::codegen::rust::call_signature_resolution::strip_collision_blocked_call_site_coercions(
                coerced,
            );
        }

        // Normalize `String::from("…").to_string()` left by stacked owned-literal paths.
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && crate::codegen::rust::string_utilities::already_owned_string_expr(coerced)
            && !crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
                &text_sig,
                arg_index,
                receiver_type_name,
            )
        {
            *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(coerced);
        }

        // Terminal: recursive same-fn call into an owned formal emitted by *this*
        // function — strip `&` re-applied by later text/forwarder/vec finalize
        // (ReBAC `resolve_check(&policy)` into `policy: Policy`).
        self.strip_recursive_owned_formal_stale_borrow(coerced, arg_expr, callee_name);

        // Terminal: `vec[i]` into owned non-Copy / Custom formals — reconcile may strip
        // IR `.clone()` when a stale Borrowed snapshot briefly applies `&` then peels it
        // (`col_string(rows[0], …)` E0507). Reuse the same IR helper as call-site coerce.
        if matches!(arg_expr, Expression::Index { .. }) {
            *coerced = self.ensure_owned_move_clone_for_reuse(
                arg_expr,
                coerced,
                &sig,
                param_idx,
            );
        }

        if std::env::var_os("WJ_DEBUG_COLLISION_BORROW").is_some()
            && callee_name.contains("graph_vertex_i64_get")
            && arg_index == 0
        {
            eprintln!(
                "WJ_DEBUG_COLLISION_BORROW reconcile_end callee={callee_name} coerced={coerced}"
            );
        }
    }

    /// Strip stale `&` on recursive calls into owned formals this function emits.
    fn strip_recursive_owned_formal_stale_borrow(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        let recursive = self.current_function_name.as_deref().is_some_and(|cur| {
            callee_name == cur || callee_name.rsplit("::").next().is_some_and(|s| s == cur)
        });
        if recursive
            && self.current_function_params.iter().any(|p| p.name == *name)
            && !self.emitted_rust_ref_formals.contains(name)
            && !self.str_ref_optimized_params.contains(name.as_str())
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            *coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        }
    }

    /// Identifier / local already typed as `usize` — skip `as usize` / `_usize`.
    pub(crate) fn arg_expression_already_usize(&self, arg: &Expression<'ast>) -> bool {
        match arg {
            Expression::Identifier { name, .. } => {
                self.current_function_params
                    .iter()
                    .find(|p| p.name == *name)
                    .is_some_and(|p| matches!(&p.type_, Type::Custom(n) if n == "usize"))
                    || self
                        .local_var_types
                        .get(name)
                        .is_some_and(|t| matches!(t, Type::Custom(n) if n == "usize"))
                    || self.usize_variables.contains(name)
            }
            _ => self.infer_expression_type_is_usize(arg) || self.expression_produces_usize(arg),
        }
    }

    /// Runtime/stdlib fallback declares `usize` at `pidx` while the WJ stub may still say `int`.
    fn fallback_signature_param_is_usize(
        &self,
        callee_name: &str,
        simple: &str,
        pidx: usize,
    ) -> bool {
        let is_usize_slot = |sig: &crate::analyzer::FunctionSignature| {
            sig.formal_param_type(pidx)
                .or_else(|| sig.param_types.get(pidx))
                .is_some_and(crate::codegen::rust::type_casting::type_is_usize)
        };
        for reg in [
            self.global_signature_registry.as_deref(),
            Some(&self.signature_registry),
            Some(crate::analyzer::SignatureRegistry::stdlib()),
        ]
        .into_iter()
        .flatten()
        {
            for key in [callee_name, simple] {
                if reg
                    .get_fallback_signature(key)
                    .or_else(|| reg.get_signature(key))
                    .is_some_and(is_usize_slot)
                {
                    return true;
                }
            }
        }
        false
    }

    /// `usize` index/capacity and int→float casts from the resolved formal type.
    fn apply_post_ir_numeric_formal_casts(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
        receiver_type_name: Option<&str>,
    ) {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        // Prefer receiver-qualified formals so suffix-ambiguous methods
        // (`insert` → Vec vs HashMap) never drive numeric casts from the wrong sig.
        let user_argc = sig
            .param_types
            .len()
            .saturating_sub(usize::from(sig.has_self_receiver_slot()));
        let cast_sig = receiver_type_name
            .and_then(|rt| self.resolve_method_function_signature(rt, simple, user_argc))
            .unwrap_or_else(|| sig.clone());
        let pidx = cast_sig.arg_param_index(arg_index);
        let formal = cast_sig
            .formal_param_type(pidx)
            .or_else(|| cast_sig.param_types.get(pidx));
        let mut fallback_usize_formal = None;
        let formal_for_usize = if formal.is_some_and(crate::codegen::rust::type_casting::type_is_usize)
        {
            formal
        } else if self.fallback_signature_param_is_usize(callee_name, simple, pidx) {
            fallback_usize_formal = Some(Type::Custom("usize".to_string()));
            fallback_usize_formal.as_ref()
        } else if formal
            .is_some_and(crate::codegen::rust::type_casting::type_is_wj_int_formal)
            && self.fallback_signature_param_is_usize(callee_name, simple, pidx)
        {
            fallback_usize_formal = Some(Type::Custom("usize".to_string()));
            fallback_usize_formal.as_ref()
        } else {
            formal
        };
        let already_usize = if formal_for_usize.is_some() {
            let expr_is_wj_int = match arg_expr {
                Expression::Identifier { name, .. } => self
                    .current_function_params
                    .iter()
                    .find(|p| p.name == *name)
                    .map(|p| &p.type_)
                    .or_else(|| self.local_var_types.get(name))
                    .is_some_and(|t| crate::codegen::rust::type_casting::type_is_wj_int_formal(t)),
                _ => self
                    .infer_expression_type(arg_expr)
                    .as_ref()
                    .is_some_and(crate::codegen::rust::type_casting::type_is_wj_int_formal),
            };
            if expr_is_wj_int {
                false
            } else {
                self.arg_expression_already_usize(arg_expr)
            }
        } else {
            self.arg_expression_already_usize(arg_expr)
        };
        crate::codegen::rust::type_casting::coerce_arg_str_for_usize_formal(
            arg_expr,
            coerced,
            formal_for_usize,
            already_usize,
        );
        // Numeric inference may have already emitted `1_usize` from a Vec::insert
        // suffix match; undo when the receiver-qualified formal is not usize.
        crate::codegen::rust::type_casting::strip_erroneous_usize_suffix_for_non_usize_formal(
            arg_expr, coerced, formal,
        );

        let skip_cast = self.should_skip_int_to_float_auto_cast_with_global(
            receiver_type_name,
            simple,
            Some(callee_name),
        );
        if skip_cast {
            return;
        }
        let Some(param_ty) = cast_sig
            .param_type_for_arg(arg_index)
            .or_else(|| cast_sig.formal_param_type(pidx))
            .or_else(|| cast_sig.param_types.get(pidx))
        else {
            return;
        };
        let arg_ty = self.infer_expression_type(arg_expr);
        crate::codegen::rust::type_classification_utilities::maybe_cast_int_arg_to_float(
            coerced,
            arg_expr,
            param_ty,
            arg_ty.as_ref(),
        );
    }

    /// Multi-candidate owned-formal peel: any defining-module / global / local
    /// signature that confirms owned emission wins over stale Borrowed stubs.
    /// Signature-driven only — no method-name ownership heuristics.
    pub(crate) fn peel_stale_borrow_for_multi_candidate_owned_formal(
        &self,
        coerced: &mut String,
        callee_name: &str,
        arg_index: usize,
        primary_sig: &crate::analyzer::FunctionSignature,
        registry: &SignatureRegistry,
        wants_mut: bool,
    ) {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let refreshed =
            self.refreshed_call_site_sig_for_arg(registry, callee_name, arg_index, primary_sig);
        let candidates: [Option<&crate::analyzer::FunctionSignature>; 8] = [
            registry.get_signature(callee_name),
            registry.get_signature(simple),
            registry.find_unique_signature_ending_with(simple),
            self.global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(callee_name)),
            self.global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(simple)),
            self.global_signature_registry
                .as_ref()
                .and_then(|g| g.find_unique_signature_ending_with(simple)),
            Some(primary_sig),
            Some(&refreshed),
        ];
        let mut_arg_emitted = self
            .function_emitted_mut_arg_indices
            .get(callee_name)
            .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
            .is_some_and(|indices| indices.contains(&arg_index));
        let any_emitted_owned = candidates.iter().flatten().any(|sig| {
            let pidx = sig.arg_param_index(arg_index);
            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
        });
        let any_emits_shared = candidates.iter().flatten().any(|sig| {
            let pidx = sig.arg_param_index(arg_index);
            crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, pidx)
        });
        let any_expects_owned = candidates.iter().flatten().any(|sig| {
            let pidx = sig.arg_param_index(arg_index);
            crate::ir::signature_bridge::call_site_expects_owned_pass(sig, pidx)
        });
        let any_expects_mut = wants_mut
            || mut_arg_emitted
            || candidates.iter().flatten().any(|sig| {
                let pidx = sig.arg_param_index(arg_index);
                matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, arg_index,
                    ),
                    crate::analyzer::OwnershipMode::MutBorrowed,
                ) || sig.param_types.get(pidx).is_some_and(|t| {
                    matches!(t, Type::MutableReference(_))
                }) || sig.formal_param_type(pidx).is_some_and(|t| {
                    matches!(t, Type::MutableReference(_))
                })
            });
        let peel_owned = (any_emitted_owned || (!any_emits_shared && any_expects_owned))
            && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                registry,
                callee_name,
                Some(primary_sig),
                arg_index,
            );
        // Prefer refreshed / primary for shared-ref confirmation — owned emission among
        // other layered stubs must not peel a confirmed `&str` / `&T` formal.
        let confirmed_shared_ref = {
            let rpidx = refreshed.arg_param_index(arg_index);
            let ppidx = primary_sig.arg_param_index(arg_index);
            crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &refreshed, rpidx,
            ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                primary_sig,
                ppidx,
            )
        };
        // Owned emission wins over a stale Borrowed stub among layered candidates
        // (dogfood `policy: Policy`). Never peel `&mut` when any candidate expects mut-borrow.
        if peel_owned && !any_expects_mut && !confirmed_shared_ref {
            if coerced.starts_with("&mut ") {
                if any_emitted_owned {
                    *coerced =
                        crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                            .to_string();
                }
            } else if coerced.starts_with('&') && any_emitted_owned {
                *coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                    .to_string();
            }
        }
    }

    /// Prefer-shared enforce that preserves IR-confirmed `&` (bug_e0308).
    ///
    /// Stale analyzer stubs must not strip collision-aware shared borrows when
    /// any registry view emits shared-ref for the slot.
    pub(crate) fn enforce_ir_ownership_preserving_confirmed_shared_ref(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
        registry: &SignatureRegistry,
    ) {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let pidx = sig.arg_param_index(arg_index);
        // Existing `&T` / `&mut T` bindings coerce to shared `&T` — never keep stacked `&`
        // (`run_dense(&csr)` when `csr: &mut DenseCsr` and callee emits `&DenseCsr`).
        if matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.identifier_binding_already_rust_ref(name)
        ) {
            let enforce_sig =
                crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(callee_name).cloned()),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned()),
                    Some(sig.clone()),
                    registry.get_signature(callee_name).cloned(),
                    registry.get_signature(simple).cloned(),
                ])
                .unwrap_or_else(|| sig.clone());
            let epidx = enforce_sig.arg_param_index(arg_index);
            if crate::ir::signature_bridge::call_site_expects_shared_borrow(&enforce_sig, epidx)
                || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    &enforce_sig, epidx,
                )
            {
                *coerced =
                    crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                        .to_string();
                return;
            }
        }
        let mut enforce_sig =
            crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(callee_name).cloned()),
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(simple).cloned()),
                Some(sig.clone()),
                registry.get_signature(callee_name).cloned(),
                registry.get_signature(simple).cloned(),
            ])
            .unwrap_or_else(|| sig.clone());
        enforce_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
            Some(enforce_sig),
            self.global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(callee_name))
                .or_else(|| {
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple))
                }),
            pidx,
        )
        .unwrap_or_else(|| sig.clone());
        let pidx = enforce_sig.arg_param_index(arg_index);
        let global_confirms_shared = self.global_signature_registry.as_ref().is_some_and(|g| {
            [callee_name, simple].into_iter().any(|key| {
                g.lookup_method(key).is_some_and(|gs| {
                    let gp = gs.arg_param_index(arg_index);
                    crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        gs, gp,
                    )
                })
            })
        });
        let keep_shared_ref = coerced.starts_with('&')
            && (global_confirms_shared
                || (!crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &enforce_sig, pidx,
                )
                    && !crate::codegen::rust::signature_promotion::bare_formal_is_vec_or_map(
                        &enforce_sig, pidx,
                    )
                    && !crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                        &enforce_sig, pidx,
                    )
                    && (crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        &enforce_sig,
                        pidx,
                    ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(
                        &enforce_sig,
                        pidx,
                    ))));
        if !keep_shared_ref {
            self.enforce_call_site_ownership_contract(
                coerced,
                arg_expr,
                &enforce_sig,
                pidx,
                callee_name,
                arg_index,
            );
        }
    }

    /// Peel `&mut` when the formal is an owned Copy aggregate (not a true `&mut T` slot).
    fn peel_spurious_mut_borrow_on_owned_copy_aggregate(
        &self,
        coerced: &mut String,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
    ) {
        if !coerced.starts_with("&mut ") {
            return;
        }
        let idx = sig.arg_param_index(arg_index);
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let mut_arg_emitted = self
            .function_emitted_mut_arg_indices
            .get(callee_name)
            .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
            .is_some_and(|indices| indices.contains(&arg_index));
        if mut_arg_emitted {
            return;
        }
        let Some(formal) = sig.formal_param_type(idx) else {
            return;
        };
        let bare = match formal {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        if crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
            && !crate::type_classification::is_copy_pass_by_value_formal(bare)
            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
            && !matches!(sig.param_types.get(idx), Some(Type::MutableReference(_)))
            && !matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                    sig, idx,
                ),
                crate::analyzer::OwnershipMode::MutBorrowed,
            )
        {
            *coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        }
    }

    /// Match-arm bindings into shared-ref text / `&str` formals keep a shared borrow
    /// (no `.clone()`, no owned pass). Owned `string` formals must move — do not
    /// treat bare `Type::String` as readonly (multipass match-Ok → owned helper).
    fn ensure_shared_borrow_on_match_arm_readonly_text(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        if !self.match_arm_bindings.contains(name.as_str()) {
            return;
        }
        let pidx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return;
        }
        let wants_shared = crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx)
            || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, pidx)
            || sig.param_types.get(pidx).is_some_and(|t| {
                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    || matches!(t, Type::Reference(_))
            });
        if !wants_shared {
            return;
        }
        if coerced.ends_with(".clone()") {
            crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
        }
        if !coerced.starts_with('&') {
            *coerced = format!("&{coerced}");
        }
    }

    /// Signature-driven `(wants_shared_ref, wants_owned)` for a call-site slot.
    /// Owned Copy-aggregate emission beats stale IR Ref (regression-060).
    pub(crate) fn call_site_slot_wants_ref_and_owned(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> (bool, bool) {
        let pidx = sig.arg_param_index(arg_index);
        let wants_owned =
            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
                || crate::codegen::rust::call_site_borrow::sig_formal_is_copy_aggregate_owned(
                    sig,
                    pidx,
                    |t| self.is_type_copy(t),
                );
        let wants_ref =
            crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx) && !wants_owned;
        (wants_ref, wants_owned)
    }

    /// Mixed-forwarder / owned-outer / reuse-clone / pure-forwarding strip.
    ///
    /// Uses IR `compute_coercion` for self-receiver calls, then the shared forwarder
    /// helpers. `receiver` is the method object (`None` for free-function calls).
    pub(crate) fn apply_post_ir_forwarder_owned_outer_and_reuse(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
        receiver: Option<&Expression<'ast>>,
    ) {
        let (wants_ref, wants_owned) = self.call_site_slot_wants_ref_and_owned(sig, arg_index);
        let pidx = sig.arg_param_index(arg_index);

        if let (Some(object), Expression::Identifier { name, .. }) = (receiver, arg_expr) {
            let receiver_is_self =
                crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(object);
            let caller_owned_param = self
                .current_function_params
                .iter()
                .any(|p| p.name == *name && !self.emitted_rust_ref_formals.contains(name));
            let is_mixed_forwarder = self.current_fn_mixed_forwarder_params.contains(name);
            if receiver_is_self && (caller_owned_param || is_mixed_forwarder) {
                let body: Vec<_> = self.current_function_body.iter().copied().collect();
                let if_facade_param =
                    self.param_used_in_if_with_condition_and_branches(&body, name);
                let mixed_forward_ref =
                    (is_mixed_forwarder || if_facade_param) && self.in_if_condition;
                let actual = self.infer_call_arg_actual_safety_type(arg_expr, coerced.as_str());
                let expected =
                    crate::ir::signature_bridge::safety_type_from_signature_param(sig, pidx);
                let kind = crate::ir::coercion::compute_coercion(&actual, &expected);
                if mixed_forward_ref
                    || (matches!(
                        kind,
                        crate::ir::coercion::CoercionKind::Borrow
                            | crate::ir::coercion::CoercionKind::MutBorrow
                    ) && !self.caller_owned_non_copy_formal(name)
                        && !wants_owned)
                {
                    if coerced.ends_with(".clone()") {
                        let base = coerced.trim_end_matches(".clone()").trim();
                        *coerced = format!("&{base}");
                    } else if !coerced.starts_with('&') {
                        *coerced = format!("&{coerced}");
                    }
                } else if matches!(kind, crate::ir::coercion::CoercionKind::Identity)
                    && coerced.starts_with('&')
                    && !coerced.starts_with("&mut ")
                    && self.caller_owned_non_copy_formal(name)
                {
                    *coerced = coerced.trim_start_matches('&').to_string();
                } else if wants_owned
                    && !mixed_forward_ref
                    && !self.current_fn_forward_ref_if_params.contains(name)
                    && coerced.starts_with('&')
                    && !coerced.starts_with("&mut ")
                {
                    let base = coerced.trim_start_matches('&');
                    let copy_aggregate = self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && self.is_type_copy(&p.type_)
                            && !crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
                    });
                    *coerced = if copy_aggregate || base.ends_with(".clone()") {
                        base.trim_end_matches(".clone()").trim().to_string()
                    } else {
                        format!("{base}.clone()")
                    };
                }
            }
            self.apply_forward_ref_and_mixed_forwarder_call_coercion(
                coerced,
                arg_expr,
                Some(object),
                wants_ref,
                wants_owned,
            );
        }

        self.finalize_owned_outer_formal_call_arg(coerced, arg_expr, wants_ref, wants_owned);

        if wants_owned && !wants_ref && !coerced.ends_with(".clone()") && !coerced.starts_with('&')
        {
            if let Expression::Identifier { name, .. } = arg_expr {
                // `&Copy` loop elems already owned via `*binding` — never append `.clone()`
                // (`*post.clone()` is E0614: clone autoderefs to i64).
                if self.borrowed_iterator_vars.contains(name)
                    && !coerced.starts_with('*')
                    && !self.binding_is_copy_pass_by_value_scalar(name)
                    && !crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                        self.current_function_return_type.as_ref(),
                    )
                {
                    *coerced = format!("{coerced}.clone()");
                } else if !self.match_arm_bindings.contains(name.as_str()) {
                    let needs_reuse_clone = self
                        .auto_clone_analysis
                        .as_ref()
                        .is_some_and(|a| a.needs_clone(name, self.current_statement_idx).is_some());
                    let skip_self = receiver.is_some_and(|object| {
                        crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(
                            object,
                        )
                    });
                    if needs_reuse_clone && self.caller_owned_non_copy_formal(name) && !skip_self {
                        *coerced = format!("{coerced}.clone()");
                    }
                }
            }
        }

        if self.in_if_condition
            && wants_owned
            && !wants_ref
            && !coerced.ends_with(".clone()")
            && !coerced.starts_with('&')
        {
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.caller_owned_non_copy_formal(name) {
                    *coerced = format!("{coerced}.clone()");
                }
            }
        }

        self.maybe_pure_forwarding_strip_call_arg(
            coerced,
            arg_expr,
            None,
            None,
            Some(arg_index),
            None,
            Some(sig),
        );
    }

    /// Peel `&` / `(&x)` when a Copy-aggregate caller binding is passed into an
    /// owned Copy-aggregate formal (regression-060 `through: Lsn` → `other: Lsn`).
    ///
    /// Collection-key slots keep `&K`. Signature-driven — no method-name lists.
    pub(crate) fn peel_copy_aggregate_caller_into_owned_callee(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
        receiver_type_name: Option<&str>,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        let trimmed = coerced.trim();
        if trimmed.starts_with("&mut ") {
            return;
        }
        if !trimmed.starts_with('&') && !(trimmed.starts_with('(') && trimmed.contains('&')) {
            return;
        }
        if crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
            sig,
            arg_index,
            receiver_type_name,
        ) {
            return;
        }
        let caller_copy = self.current_function_params.iter().any(|p| {
            p.name == *name
                && crate::codegen::rust::call_site_borrow::bare_type_is_copy_aggregate_owned_formal(
                    &p.type_,
                    |ty| self.is_type_copy(ty),
                )
        });
        if !caller_copy {
            return;
        }
        let param_idx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
            sig, param_idx,
        ) {
            return;
        }
        let callee_copy = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
            sig, param_idx,
        ) || crate::codegen::rust::call_site_borrow::sig_formal_is_copy_aggregate_owned(
            sig,
            param_idx,
            |ty| self.is_type_copy(ty),
        ) || sig
            .formal_param_type(param_idx)
            .or_else(|| sig.param_types.get(param_idx))
            .is_some_and(|t| {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                crate::codegen::rust::call_site_borrow::bare_type_is_copy_aggregate_owned_formal(
                    bare,
                    |ty| self.is_type_copy(ty),
                )
            });
        if !callee_copy {
            return;
        }
        let mut s = coerced.trim().to_string();
        loop {
            if s.starts_with('(') && s.ends_with(')') {
                let inner = s[1..s.len() - 1].trim().to_string();
                if inner.starts_with('&') || inner.starts_with("&mut ") {
                    s = inner;
                    continue;
                }
            }
            if s.starts_with("&mut ") {
                break;
            }
            if s.starts_with('&') {
                s = s[1..].trim().to_string();
                continue;
            }
            break;
        }
        *coerced = s;
    }

    /// Uses `emitted_owned_arg_contract` to handle stale analyzer metadata.
    pub(crate) fn enforce_call_site_ownership_contract(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        sig: &crate::analyzer::FunctionSignature,
        param_idx: usize,
        callee_name: &str,
        arg_index: usize,
    ) {
        if let Expression::Identifier { name, .. } = arg_expr {
            if (self.borrowed_iterator_vars.contains(name)
                || self
                    .local_var_types
                    .get(name)
                    .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_))))
                && crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                    self.current_function_return_type.as_ref(),
                )
            {
                crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
                return;
            }
        }
        let runtime_std_borrow =
            crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                &self.signature_registry,
                callee_name,
                Some(sig),
                arg_index,
            );
        let emits_shared_ref =
            crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                sig, param_idx,
            );
        let global_emits_shared_ref = {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            self.global_signature_registry.as_ref().is_some_and(|g| {
                [callee_name, simple].into_iter().any(|key| {
                    g.lookup_method(key).is_some_and(|gs| {
                        crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            gs,
                            gs.arg_param_index(arg_index),
                        )
                    })
                })
            })
        };
        let mut_arg_emitted = {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            self.function_emitted_mut_arg_indices
                .get(callee_name)
                .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
                .is_some_and(|indices| indices.contains(&arg_index))
        };
        let expects_mut = mut_arg_emitted
            || matches!(
                sig.param_types.get(param_idx),
                Some(Type::MutableReference(_))
            )
            || matches!(
                sig.formal_param_type(param_idx),
                Some(Type::MutableReference(_))
            )
            || matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                    sig, param_idx,
                ),
                crate::analyzer::OwnershipMode::MutBorrowed,
            );
        // Registry-aware Copy aggregates (Lsn, …) always emit owned formals — strip over-borrow
        // even when `emitted_owned_arg_contract` lacks pure-analysis Copy knowledge (regression-060).
        // Stale `Reference(Lsn)` in formal_param_types must not block this: formal generation
        // strips Copy-aggregate `&T` while analyzer metadata may still wrap the type.
        // Never treat true `&mut T` / MutBorrowed slots as owned Copy (apply_rotation /
        // fill_grid / update_health_regen).
        let copy_aggregate_owned = !expects_mut
            && sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
                .is_some_and(|t| {
                    if matches!(t, Type::MutableReference(_)) {
                        return false;
                    }
                    let bare = match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    self.is_type_copy(bare)
                        && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                })
            && !emits_shared_ref
            && !global_emits_shared_ref;
        let force_owned = !expects_mut
            && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                sig, param_idx,
            ) || copy_aggregate_owned
                || crate::codegen::rust::signature_promotion::bare_formal_is_vec_or_map(
                    sig, param_idx,
                )
                || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                    sig, param_idx,
                ))
            && !runtime_std_borrow
            // Never strip `&` for text formals the callee emits as `&str` / shared ref
            // (regression-049 `replay_to_lsn(&self.path)`).
            && !emits_shared_ref
            && !global_emits_shared_ref;
        // Owned emission wins over stale analyzer/IR Ref expectations (regression-060
        // `is_at_or_before(&through)` → `other: Lsn`). Strip before shared-borrow path.
        // Do not strip `.clone()` — Copy aggregates still need multi-use clones (dogfood seed_write).
        // Also peel parenthesized unary refs from expression codegen: `(&through)`.
        if force_owned {
            let before = coerced.trim().to_string();
            let mut s = before.clone();
            loop {
                if s.starts_with('(') && s.ends_with(')') {
                    let inner = s[1..s.len() - 1].trim().to_string();
                    if inner.starts_with('&') || inner.starts_with("&mut ") {
                        s = inner;
                        continue;
                    }
                }
                if s.starts_with("&mut ") {
                    s = s["&mut ".len()..].trim().to_string();
                    continue;
                }
                if s.starts_with('&') {
                    s = s[1..].trim().to_string();
                    continue;
                }
                break;
            }
            let expected = safety_type_from_signature_param(sig, param_idx);
            let actual = self.infer_call_arg_actual_safety_type(arg_expr, before.as_str());
            if matches!(compute_coercion(&actual, &expected), CoercionKind::Clone)
                && !s.ends_with(".clone()")
                && !s.ends_with(".to_string()")
                && !s.ends_with(".to_owned()")
            {
                s = format!("{s}.clone()");
            }
            *coerced = s;
            return;
        }
        if crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, param_idx) {
            // Owned clones/`to_string` already satisfy `&str` / `&T` via deref —
            // never emit `&x.clone()` (E0308 into owned String, redundant for `&str`).
            if coerced.ends_with(".clone()")
                || coerced.ends_with(".to_string()")
                || coerced.ends_with(".to_owned()")
            {
                return;
            }
            if matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.identifier_binding_already_rust_ref(name)
            ) {
                // `&mut T` coerces to `&T` — never stack shared `&` (`&&mut`, `&&T`).
                *coerced =
                    crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                        .to_string();
                return;
            }
            if !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
            {
                *coerced = format!("&{coerced}");
            }
            return;
        }
        let expected = safety_type_from_signature_param(sig, param_idx);
        let actual = self.infer_call_arg_actual_safety_type(arg_expr, coerced.as_str());
        crate::ir::coercion::enforce_ownership_contract_on_coerced_arg_with_force_owned(
            coerced,
            &actual,
            &expected,
            force_owned,
            false,
            runtime_std_borrow,
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
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.identifier_binding_already_rust_ref(name)
                || self.inferred_borrowed_params.contains(name)
                || self.str_ref_optimized_params.contains(name)
            {
                return;
            }
        }
        let Some(_rt) = receiver_type_name else {
            return;
        };
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
            let qualified = format!("{rt}::{method}");
            let initial = self.resolve_method_function_signature(rt, method, arg_count);
            let Some(sig) =
                crate::codegen::rust::signature_promotion::refresh_call_site_signature_for_arg(
                    initial,
                    &qualified,
                    arg_index,
                    self.global_signature_registry.as_deref(),
                    &self.signature_registry,
                )
            else {
                continue;
            };
            let pidx = sig.arg_param_index(arg_index);
            // Emitted owned formals win over stale MutBorrowed/Borrowed metadata.
            if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(&sig, pidx) {
                if coerced.starts_with('&') {
                    *coerced =
                        crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                            coerced,
                        );
                }
                return;
            }
            let wants_mut = sig
                .param_types
                .get(pidx)
                .is_some_and(|t| matches!(t, Type::MutableReference(_)))
                || matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                    &sig, arg_index,
                ),
                crate::analyzer::OwnershipMode::MutBorrowed
            );
            if wants_mut {
                // String literals are never `&mut` lvalues.
                if crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr) {
                    return;
                }
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
            if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &sig, pidx,
            ) && !coerced.starts_with('&')
            {
                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(coerced);
                return;
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
        if let Some(resolved) =
            self.resolve_call_signature_with_global(method, receiver_type_name, arg_count)
        {
            let pidx = resolved.sig.arg_param_index(arg_index);
            if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                &resolved.sig,
                pidx,
            ) {
                return;
            }
            if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &resolved.sig,
                pidx,
            ) {
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
        if ownership_from_rust_expr(coerced.as_str()).is_some() {
            return coerced;
        }
        if coerced.starts_with('&') {
            return coerced;
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            // Only skip when Rust already emits this binding as a reference formal.
            // `identifier_already_ref` is too broad for match-arm payloads and
            // stale inferred_borrowed_params (json::get(&v) needs & on owned `v`).
            if self.emitted_rust_ref_formals.contains(name)
                || self.str_ref_optimized_params.contains(name)
                || self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && matches!(p.type_, Type::Reference(_) | Type::MutableReference(_))
                })
            {
                return coerced;
            }
        }
        let module = callee_name.split("::").next().unwrap_or("");
        let method = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let inferred_type = self.infer_expression_type(arg_expr);
        let effective_module =
            crate::codegen::rust::stdlib_method_traits::resolve_runtime_std_module(
                module,
                receiver_type_name,
            );

        if !coerced.starts_with('&')
            && (crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                &self.signature_registry,
                callee_name,
                signature,
                arg_index,
            )
                || crate::codegen::rust::stdlib_method_traits::runtime_std_call_arg_needs_auto_borrow(
                    effective_module,
                    method,
                    signature,
                    arg_index,
                    inferred_type.as_ref(),
                    arg_expr,
                    receiver_type_name,
                ))
        {
            coerced = crate::ir::target_encodings::rust_shared_borrow(&coerced);
        }

        let signature_for_borrow =
            if crate::codegen::rust::call_signature_resolution::is_external_module_qualified_call(
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
                            if self.binding_emits_as_rust_shared_ref(name)
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
                                if self.binding_emits_as_rust_shared_ref(name)
                        ) {
                            coerced =
                                crate::ir::target_encodings::rust_shared_borrow(&coerced);
                        }
                    }
                }
            }
        }

        if matches!(
                arg_expr,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            )
            && coerced.ends_with(".to_string()")
            && crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
                signature,
                arg_index,
            )
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
            if self.copy_match_payload_binding(name) {
                return coerced[1..].to_string();
            }
            let is_copy = self
                .local_var_types
                .get(name)
                .is_some_and(|t| self.is_type_copy(t))
                || self
                    .infer_expression_type(arg_expr)
                    .is_some_and(|t| self.is_type_copy(&t))
                || self.binding_is_copy_pass_by_value_scalar(name);
            if is_copy {
                // `&binding.clone()` → `*binding` (never `*binding.clone()`, E0614 on Copy).
                let mut core = coerced[1..].to_string();
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut core);
                if core.starts_with('*') {
                    return core;
                }
                return format!("*{core}");
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

        // Generated owned String (`"lit".to_string()` / `.to_owned()`) must not be
        // classified as a borrowed string-literal — that causes a later `&` prefix
        // (`&"lit".to_string()`) for Owned formals (Objective::kill factory).
        if arg_str.ends_with(".to_string()") || arg_str.ends_with(".to_owned()") {
            return SafetyType::owned(BaseType::String);
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

        if matches!(
            arg_expr,
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                operand: _,
                ..
            }
        ) {
            if let Some(inner_ty) = self.infer_expression_type(arg_expr) {
                return safety_type_from_parser_type(&inner_ty, None);
            }
        }

        if let Expression::Identifier { name, .. } = arg_expr {
            if let Some(param) = self
                .current_function_params
                .iter()
                .find(|p| p.name == *name)
            {
                if self.is_type_copy(&param.type_)
                    && !crate::type_classification::is_copy_pass_by_value_formal(&param.type_)
                {
                    return SafetyType::copy(crate::ir::node::parser_type_to_base_type(
                        &param.type_,
                    ));
                }
            }
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
                return self
                    .safety_type_for_param_binding(arg_expr, OwnedType::MutRef(Region::fresh(1)));
            }
            if self.identifier_already_ref(name) {
                return self
                    .safety_type_for_param_binding(arg_expr, OwnedType::Ref(Region::fresh(0)));
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

        // Copy field reads through `&Struct` / match bindings are values in Rust
        // (`failure.status` → `i64`), not `&i64`. Treating them as Ref makes
        // ownership coercion emit `*failure.status` (E0614).
        if matches!(arg_expr, Expression::FieldAccess { .. }) {
            if let Some(ty) = self.infer_expression_type(arg_expr) {
                let bare = match &ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                if self.is_type_copy(bare) {
                    return SafetyType::copy(crate::ir::node::parser_type_to_base_type(bare));
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
                    }) || self.ir_callee_arg_expects_shared_borrow(
                        &self.signature_registry,
                        callee,
                        idx,
                        None,
                        None,
                    ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                        self.ir_callee_arg_expects_shared_borrow(g, callee, idx, None, None)
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
                    }) || self.ir_callee_arg_expects_shared_borrow(
                        &self.signature_registry,
                        callee,
                        idx,
                        None,
                        None,
                    ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                        self.ir_callee_arg_expects_shared_borrow(g, callee, idx, None, None)
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
            // Still rewrite clone → mem::take for call-arg writeback behind &mut self.
            if let Some(rewritten) = self.try_self_field_writeback_owned_arg(arg_expr, arg_str) {
                return rewritten;
            }
            return arg_str.to_string();
        }
        if let Some(rewritten) = self.try_self_field_writeback_owned_arg(arg_expr, arg_str) {
            return rewritten;
        }
        let Some(path) = Self::auto_clone_expr_path(arg_expr) else {
            return arg_str.to_string();
        };
        let needs = self
            .auto_clone_analysis
            .as_ref()
            .is_some_and(|a| a.needs_clone(&path, self.current_statement_idx).is_some());
        if needs {
            // Only scalar Copy (i64/bool/…) skip clone; Copy aggregates/enums still need
            // `.clone()` on multi-use owned moves (regression-063 Value).
            let skip = match arg_expr {
                Expression::Identifier { name, .. } => {
                    self.binding_is_copy_pass_by_value_scalar(name)
                }
                _ => self.infer_expression_type(arg_expr).is_some_and(|t| {
                    let bare = match &t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    crate::type_classification::is_copy_pass_by_value_formal(bare)
                }),
            };
            if !skip {
                // Clone yields an owned value. If a prior pass prefixed `&`, drop it
                // (`&stack.item.id.clone()` is `&String`, not `String`).
                let owned = if arg_str.starts_with("&mut ") {
                    arg_str
                } else {
                    arg_str.trim_start_matches('&')
                };
                return format!("{owned}.clone()");
            }
        }
        arg_str.to_string()
    }

    pub(in crate::codegen::rust) fn auto_clone_expr_path(
        expr: &Expression<'ast>,
    ) -> Option<String> {
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

    pub(crate) fn ir_sig_arg_expects_shared_borrow(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> bool {
        let pidx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return false;
        }
        if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string_for_call_arg(
            sig, arg_index,
        ) && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, pidx)
        {
            return false;
        }
        if sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(pidx))
            .copied()
            == Some(false)
        {
            return false;
        }
        if let Some(bare) = sig
            .formal_param_type(pidx)
            .or_else(|| sig.param_types.get(pidx))
            .map(|t| match t {
                Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                other => other,
            })
        {
            // Copy aggregates without confirmed shared-ref emission usually stay owned
            // (Lsn). Defer to the signature bridge so analyzer-Borrowed bare Custom that
            // the bridge still marks Ref (WDB-097 DenseCsr; also when field layout is
            // unknown and `is_type_copy` is a false positive) is not demoted to Identity.
            if self.is_type_copy(bare)
                && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    sig, pidx,
                )
            {
                return crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx);
            }
        }
        crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx)
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
        // When `local_sig` is present it is authoritative (fn-pointer / call-resolved).
        if let Some(sig) = local_sig {
            if self.ir_callee_arg_emits_owned_contract(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                Some(sig),
            ) {
                return false;
            }
            return self.ir_sig_arg_expects_shared_borrow(sig, arg_index);
        }
        if self.ir_callee_arg_emits_owned_contract(
            registry,
            callee_name,
            arg_index,
            user_arg_count,
            None,
        ) {
            return false;
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
            // Qualified callees must not fall back to bare simple-name globals
            // (homonym ownership from a different module).
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
        // Call-resolved / fn-pointer signatures are authoritative. Registry
        // homonyms (e.g. another `has_item` with owned `string`) must not force
        // `.clone()` into borrowed formals.
        if let Some(sig) = local_sig {
            return check(sig);
        }
        if registry.get_signature(callee_name).is_some_and(check) {
            return true;
        }
        // Do not consult bare `simple` for `module::fn` / `Type::method` — that
        // applies ownership from an unrelated homonym (qualified-call string bug).
        if let Some(global) = self.global_signature_registry.as_ref() {
            if global.get_signature(callee_name).is_some_and(check) {
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
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return false;
        }
        // Stale analyzer `MutableReference` on owned Copy aggregate formals (trait
        // `set_camera(camera: CameraData)`) must not force `&mut` at call sites when
        // defining-module emission kept an owned formal (no `function_emitted_mut_arg_indices` slot).
        if sig.formal_param_type(pidx).is_some_and(|formal| {
            !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                && self.is_type_copy(formal)
                && !crate::type_classification::is_copy_pass_by_value_formal(formal)
        }) {
            let simple = sig.name.rsplit("::").next().unwrap_or(sig.name.as_str());
            let defining_module_emits_mut = self
                .function_emitted_mut_arg_indices
                .get(&sig.name)
                .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
                .is_some_and(|indices| indices.contains(&arg_index));
            if !defining_module_emits_mut {
                return false;
            }
        }
        sig.param_types
            .get(pidx)
            .is_some_and(|t| matches!(t, Type::MutableReference(_)))
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
            // Stale multipass slots can linger; prefer emitted owned contract.
            let owned_emitted = local_sig
                .map(|sig| {
                    let pidx = sig.arg_param_index(arg_index);
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
                })
                .or_else(|| {
                    registry.get_signature(callee_name).map(|sig| {
                        let pidx = sig.arg_param_index(arg_index);
                        crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig, pidx,
                        )
                    })
                })
                .unwrap_or(false);
            if !owned_emitted {
                return true;
            }
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
    let trimmed = expr.trim();
    if trimmed.starts_with("&mut ") {
        Some(OwnedType::MutRef(Region::fresh(1)))
    } else if trimmed.starts_with('&') {
        Some(OwnedType::Ref(Region::fresh(0)))
    } else if trimmed.starts_with('(') && trimmed.ends_with(')') {
        ownership_from_rust_expr(&trimmed[1..trimmed.len() - 1])
    } else if trimmed.ends_with(".to_string()") || trimmed.ends_with(".to_owned()") {
        Some(OwnedType::Owned)
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

#[cfg(test)]
mod ir_total_tests {
    use super::*;
    use crate::analyzer::{Analyzer, OwnershipMode};
    use crate::codegen::rust::{CodeGenerator, IrCutoverConfig};
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::CompilationTarget;

    #[test]
    fn apply_ir_returns_some_when_call_sites_on_for_known_callee() {
        let source = r#"
fn takes_borrowed(s: string) {}
fn main() {
    let x = "hi"
    takes_borrowed(x)
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, mut registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let _ = analyzed;
        // Ensure Borrowed formal is visible to IR.
        let sig = registry
            .signatures
            .get_mut("takes_borrowed")
            .expect("takes_borrowed signature from analyzer");
        sig.param_ownership = vec![OwnershipMode::Borrowed];
        sig.emitted_rust_ref_params = Some(vec![true]);

        let mut gen = CodeGenerator::new(registry.clone(), CompilationTarget::Rust);
        gen.ir_cutover = IrCutoverConfig {
            ownership: true,
            clones: true,
            param_types: true,
            str_ref: true,
            call_sites: true,
            locals: true,
        };

        let call_arg = program
            .items
            .iter()
            .find_map(|item| {
                if let crate::parser::Item::Function { decl, .. } = item {
                    if decl.name != "main" {
                        return None;
                    }
                    decl.body.iter().find_map(|stmt| {
                        if let crate::parser::Statement::Expression { expr, .. } = stmt {
                            if let Expression::Call { arguments, .. } = expr {
                                return arguments.first().map(|(_, a)| *a);
                            }
                        }
                        None
                    })
                } else {
                    None
                }
            })
            .expect("call arg");

        let coerced = gen.apply_ir_call_site_coercion(
            &registry,
            "takes_borrowed",
            0,
            call_arg,
            "x",
            registry.get_signature("takes_borrowed"),
            None,
            Some(1),
        );
        assert!(
            coerced.is_some(),
            "IR must always coerce known callees when call_sites is on"
        );
    }

    #[test]
    fn module_boundary_callee_detection() {
        assert!(CodeGenerator::is_module_boundary_callee(
            "unknown_crate::missing_api"
        ));
        assert!(CodeGenerator::is_module_boundary_callee(
            "wal::replay_to_lsn"
        ));
        assert!(!CodeGenerator::is_module_boundary_callee("HashMap::new"));
        assert!(!CodeGenerator::is_module_boundary_callee("Self::new"));
        assert!(!CodeGenerator::is_module_boundary_callee("plain_fn"));
        // Same-crate paths resolve via bare-name registry entries (WDB-094).
        assert!(!CodeGenerator::is_module_boundary_callee(
            "crate::circuit_row::copy_rows"
        ));
        assert!(!CodeGenerator::is_module_boundary_callee(
            "crate::vec_map::vec_map_get_f64"
        ));
    }
}
