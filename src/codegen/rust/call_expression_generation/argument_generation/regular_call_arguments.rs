//! Plain function call argument lowering (ownership, FFI, casts).

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::super::CodeGenerator;

#[allow(clippy::too_many_lines)]
pub(in crate::codegen::rust) fn collect_regular_function_arguments<'ast>(
    gen: &mut CodeGenerator<'ast>,
    func_name: &str,
    _func_str: &str,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
    signature: &Option<crate::analyzer::FunctionSignature>,
    _signature_from_simple_fallback: bool,
    is_extern_call: bool,
) -> Vec<String> {
    let associated_receiver: Option<String> =
        if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
            func_name,
        ) {
            func_name
                .rsplit_once("::")
                .map(|(receiver, _)| receiver.to_string())
        } else {
            None
        };
    arguments
        .iter()
        .enumerate()
        .flat_map(|(i, (_label, arg))| {
            // CRITICAL: Reset in_field_access_object for argument generation.
            // Arguments are independent expressions, NOT part of a field/method/index chain.
            // Without this, `process_property(prop.name, prop.value).as_str()` would
            // leak in_field_access_object from the MethodCall handler into prop.name/prop.value,
            // suppressing necessary .clone() calls.
            let param_expects_borrowed = signature.as_ref().is_some_and(|sig| {
                let idx = sig.arg_param_index(i);
                matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, i,
                    ),
                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                ) || sig.param_types.get(idx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        || crate::codegen::rust::string_utilities::param_is_rust_string_ref(t)
                })
            });
            let prev_suppress = gen.suppress_borrowed_clone;
            if param_expects_borrowed
                && matches!(
                    arg,
                    Expression::FieldAccess { .. } | Expression::Identifier { .. }
                )
            {
                gen.suppress_borrowed_clone = true;
            }
            // Signature-driven nested context (e.g. `vec![tenant_id, …]` into
            // `params: Vec<string>`) — same IR element coercion as method args.
            let prev_call_arg_expected = gen.call_arg_expected_type.clone();
            let prev_arg_float_target = gen.assignment_float_target_type.clone();
            let prev_arg_int_target = gen.assignment_int_target_type.clone();
            if let Some(ref sig) = signature {
                let pidx = sig.arg_param_index(i);
                let param_ty = sig
                    .param_type_for_arg(i)
                    .or_else(|| sig.formal_param_type(pidx))
                    .or_else(|| sig.param_types.get(pidx))
                    .cloned();
                if param_ty.as_ref().is_some_and(
                    crate::codegen::rust::type_classification_utilities::is_float_type,
                ) && gen.assignment_float_target_type.is_none()
                {
                    gen.assignment_float_target_type = param_ty.clone();
                }
                if let Some(peer) =
                    crate::codegen::rust::type_casting::assignment_int_peer_from_formal(
                        param_ty.as_ref(),
                    )
                {
                    gen.assignment_int_target_type = Some(peer);
                }
                if let Some(ty) = param_ty {
                    gen.call_arg_expected_type = Some(ty);
                }
            }
            // P3_ATOMIC_I64_ASSOCIATED_OWNER_PEER: AtomicI64::new(0) owner width beats void/i32-coord let context.
            if let Some(peer) = associated_receiver.as_deref().and_then(|n| {
                crate::codegen::rust::type_casting::assignment_int_peer_from_owner_type_name(n)
            }) {
                gen.assignment_int_target_type = Some(peer);
            }
            let scope = gen.arg_gen_scope();
            let mut arg_str = gen.generate_expression(arg);
            gen.restore_arg_gen_scope(scope);
            gen.assignment_float_target_type = prev_arg_float_target;
            gen.assignment_int_target_type = prev_arg_int_target;
            gen.call_arg_expected_type = prev_call_arg_expected;
            gen.suppress_borrowed_clone = prev_suppress;
            arg_str = gen.peel_copy_ref_match_binding_for_value(arg, &arg_str);
            if let Expression::Identifier { name, .. } = arg {
                if gen.copy_match_payload_binding(name)
                    && (arg_str.starts_with('&') || arg_str.starts_with('*'))
                {
                    while arg_str.starts_with('*') {
                        arg_str = arg_str[1..].to_string();
                    }
                    arg_str = crate::codegen::rust::expression_utilities::borrow_base_expr(
                        &arg_str,
                    )
                    .to_string();
                }
            }
            // Pre-compute ownership collision for the whole argument.
            let has_ownership_collision =
                crate::codegen::rust::call_signature_resolution::has_ownership_collision_for_call(
                    gen, func_name,
                );

            // WINDJAMMER FFI: Convert string arguments for extern functions
            if is_extern_call {
                if let Some(ref sig) = signature {
                    if let Some(param_type) = sig.param_types.get(i) {
                        if matches!(param_type, Type::Custom(name) if name == "str") {
                            // Expand str to (ptr, len)
                            return vec![
                                format!("{}.as_bytes().as_ptr()", arg_str),
                                format!("{}.as_bytes().len()", arg_str),
                            ];
                        }
                        // string/String params → FfiString via string_to_ffi
                        // TDD FIX: Always use .to_string() - infer_expression_type returns
                        // declared param type (Type::String), not actual Rust type. When
                        // ownership infers Borrowed, param becomes &str in Rust, but we
                        // thought it was String and passed directly → E0308.
                        // .to_string() works for both &str and String (String::to_string = clone).
                        //
                        // TDD FIX: Strip redundant .to_string() before wrapping.
                        // Bug: User writes render_text(label.to_string(), x, y). Expression
                        // generation produces "label.to_string()", then we added another
                        // → string_to_ffi(label.to_string().to_string()). Fix: If arg_str
                        // already ends with .to_string(), don't add another.
                        if crate::codegen::rust::string_utilities::param_is_owned_string_type(param_type)
                            || crate::codegen::rust::string_utilities::param_is_rust_str_ref(param_type)
                            || crate::codegen::rust::types::is_windjammer_text_type(param_type)
                        {
                            let mut ffi_arg = arg_str.clone();
                            gen.maybe_clone_borrowed_field_for_owned_param(arg, &mut ffi_arg);
                            let inner = if matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            ) {
                                if ffi_arg.ends_with(".to_string()") {
                                    ffi_arg.clone()
                                } else {
                                    format!("{}.to_string()", ffi_arg)
                                }
                            } else if ffi_arg.ends_with(".clone()") {
                                ffi_arg.clone()
                            } else if ffi_arg.ends_with(".to_string()") {
                                ffi_arg.clone()
                            } else {
                                format!("{}.to_string()", ffi_arg)
                            };
                            return vec![format!(
                                "windjammer_runtime::ffi::string_to_ffi({inner})"
                            )];
                        }
                        // Owned non-text extern params (e.g. Vec<u8>) — clone fields/indexes
                        // moved out from behind borrowed formals (`key.bytes` on `&Key`).
                        let expects_owned = matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                sig, i,
                            ),
                            OwnershipMode::Owned,
                        );
                        if expects_owned
                            && !crate::codegen::rust::type_analysis_pure::is_copy_type(param_type)
                        {
                            gen.maybe_clone_borrowed_field_for_owned_param(arg, &mut arg_str);
                            gen.maybe_clone_index_for_owned_param(arg, &mut arg_str);
                        } else if matches!(
                            arg,
                            Expression::FieldAccess { .. } | Expression::Index { .. }
                        ) && gen.field_access_root_is_behind_reference(arg)
                        {
                            // Even when extern metadata omits Owned, never move non-Copy
                            // fields out from behind `&T` (regression-043 / E0507).
                            let is_copy = gen
                                .infer_expression_type(arg)
                                .as_ref()
                                .is_some_and(|t| gen.is_copy_move_out_type(t));
                            if !is_copy && !arg_str.ends_with(".clone()") {
                                arg_str = format!("{}.clone()", arg_str);
                            }
                        }
                        // P3.368: i32-coord locals into u32/i64 extern formals (texture FFI, ECS).
                        let mixed_int = Some(gen.int_type_for_mixed_int_codegen(arg));
                        let arg_ty = gen.infer_expression_type(arg);
                        crate::codegen::rust::type_casting::apply_numeric_formal_coercions(
                            arg,
                            &mut arg_str,
                            Some(param_type),
                            arg_ty.as_ref(),
                            mixed_int,
                        );
                    }
                } else if matches!(
                    arg,
                    Expression::FieldAccess { .. } | Expression::Index { .. }
                ) {
                    // Extern call without resolved signature: still clone non-Copy
                    // field/index moves from behind references (regression-043).
                    if gen.field_access_root_is_behind_reference(arg) {
                        let is_copy = gen
                            .infer_expression_type(arg)
                            .as_ref()
                            .is_some_and(|t| gen.is_copy_move_out_type(t));
                        if !is_copy && !arg_str.ends_with(".clone()") {
                            arg_str = format!("{}.clone()", arg_str);
                        }
                    }
                }
            }

            // Non-Copy / Custom `vec[i]` into owned formals — clone before IR pass (E0507).
            if matches!(arg, Expression::Index { .. }) {
                gen.maybe_clone_index_for_owned_param(arg, &mut arg_str);
            }

            if let Some(ref sig) = signature {
                let pidx = sig.arg_param_index(i);
                let param_ty = sig
                    .param_type_for_arg(i)
                    .or_else(|| sig.formal_param_type(pidx))
                    .or_else(|| sig.param_types.get(pidx));
                let mixed_int = Some(gen.int_type_for_mixed_int_codegen(arg));
                let arg_ty = gen.infer_expression_type(arg);
                crate::codegen::rust::type_casting::apply_numeric_formal_coercions(
                    arg,
                    &mut arg_str,
                    param_ty,
                    arg_ty.as_ref(),
                    mixed_int,
                );
            }

            if gen.ir_cutover.call_sites && !is_extern_call {
                if let Some(mut coerced) = gen.apply_ir_call_site_coercion(
                    &gen.signature_registry,
                    func_name,
                    i,
                    arg,
                    &arg_str,
                    signature.as_ref(),
                    associated_receiver.as_deref(),
                    Some(arguments.len()),
                ) {
                    // IR coercion can miss `&` when registry metadata lags; honor resolved signature.
                    let initial_sig = associated_receiver
                        .as_ref()
                        .and_then(|rt| {
                            func_name.rsplit_once("::").and_then(|(_, method)| {
                                gen.resolve_method_function_signature(
                                    rt,
                                    method,
                                    arguments.len(),
                                )
                            })
                        })
                        .or_else(|| {
                            if func_name.starts_with("Self::") {
                                gen.current_struct_name.as_ref().and_then(|tn| {
                                    func_name.strip_prefix("Self::").and_then(|method| {
                                        gen.lookup_method_signature(tn, method)
                                            .map(|ms| ms.to_function_signature())
                                    })
                                })
                            } else {
                                None
                            }
                        });
                    let post_ir_borrow_sig = if associated_receiver.is_some()
                        || initial_sig.is_some()
                    {
                        gen.refresh_call_site_signature_for_arg(
                            initial_sig.or_else(|| signature.clone()),
                            func_name,
                            i,
                        )
                        .or_else(|| signature.clone())
                    } else {
                        gen.refresh_call_site_signature_for_arg(
                            gen.get_signature_with_global(func_name)
                                .cloned()
                                .or_else(|| signature.clone()),
                            func_name,
                            i,
                        )
                    };
                    // Mut-borrow / owned peel / prefer-shared enforce / text finalize run
                    // once at the end via `reconcile_post_ir_mut_borrow_and_owned_peel`.
                    coerced = gen.coerce_explicit_ref_for_owned_callee_arg(
                        arg,
                        coerced,
                        post_ir_borrow_sig
                            .as_ref()
                            .or(signature.as_ref()),
                        i,
                    );
                    if gen.in_user_written_closure {
                        if let Expression::Identifier { name, .. } = arg {
                            if gen.user_closure_params.contains(name)
                                && coerced.ends_with(".clone()")
                            {
                                coerced = coerced
                                    [..coerced.len() - ".clone()".len()]
                                    .to_string();
                            }
                        }
                    }
                    if let Expression::Identifier { name, .. } = arg {
                        if gen.in_user_written_closure && gen.user_closure_params.contains(name) {
                            let sig_for_closure = post_ir_borrow_sig
                                .as_ref()
                                .or(signature.as_ref())
                                .or_else(|| gen.signature_registry.get_signature(func_name))
                                .or_else(|| {
                                    gen.global_signature_registry
                                        .as_ref()
                                        .and_then(|g| g.get_signature(func_name))
                                });
                            if let Some(sig) = sig_for_closure {
                                let pidx = sig.arg_param_index(i);
                                if (crate::ir::signature_bridge::call_site_expects_shared_borrow(
                                    sig, pidx,
                                ) || sig.param_types.get(pidx).is_some_and(|t| {
                                    matches!(t, Type::Reference(_))
                                }))
                                    && !coerced.starts_with('&')
                                    && !coerced.starts_with("&mut ")
                                {
                                    coerced = format!("&{coerced}");
                                }
                            }
                        }
                    }
                    // Single terminal reconcile: prefer-shared enforce, copy-aggregate
                    // `&mut` peel, text/collection finalize, vec borrow, owned-lit peel.
                    let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                    let peel_sig = gen.refresh_call_site_signature_for_arg(
                        post_ir_borrow_sig.or_else(|| signature.clone()),
                        func_name,
                        i,
                    );
                    if let Some(sig) = peel_sig.as_ref() {
                        gen.reconcile_post_ir_mut_borrow_and_owned_peel(
                            &mut coerced,
                            arg,
                            func_name,
                            i,
                            sig,
                            &gen.signature_registry,
                            associated_receiver.as_deref(),
                            None,
                            Some(arguments.len()),
                            has_ownership_collision,
                        );
                    } else if has_ownership_collision
                        && crate::codegen::rust::call_signature_resolution::ownership_collision_blocks_autoborrow(
                            simple,
                        )
                    {
                        crate::codegen::rust::call_signature_resolution::strip_collision_blocked_call_site_coercions(
                            &mut coerced,
                        );
                    }
                    let lookup_callee = gen.signature_lookup_callee_name(func_name);
                    let dep_shared = gen.cross_crate_dep_arg_confirms_shared(func_name, i)
                        || gen.cross_crate_dep_arg_confirms_shared(lookup_callee.as_ref(), i);
                    let dep_owned = gen.cross_crate_dep_arg_confirms_owned(func_name, i)
                        || gen.cross_crate_dep_arg_confirms_owned(lookup_callee.as_ref(), i);
                    if dep_shared
                        && !coerced.starts_with('&')
                        && !coerced.starts_with("&mut ")
                    {
                        if let Expression::Identifier { name, .. } = arg {
                            // Already-`&T` caller formals reborrow as the name (`touch(csr)`).
                            if !gen.identifier_binding_already_rust_ref(name) {
                                coerced = format!("&{name}");
                            }
                        }
                    } else if (dep_owned
                        || (!func_name.contains("::")
                            && (gen.preregistered_free_call_arg_emits_owned(func_name, i)
                                || gen.preregistered_free_call_arg_emits_owned(
                                    lookup_callee.as_ref(),
                                    i,
                                ))))
                        && coerced.starts_with('&')
                        && !coerced.starts_with("&mut ")
                    {
                        coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(
                            &coerced,
                        )
                        .to_string();
                    }
                    // Terminal guard when reconcile was skipped (missing peel_sig).
                    gen.peel_stacked_amp_on_emitted_ref_binding(
                        &mut coerced,
                        arg,
                        peel_sig.as_ref(),
                        i,
                        true,
                    );
                    coerced =
                        crate::codegen::rust::string_utilities::finalize_explicit_user_clone_call_site(
                            arg,
                            &arg_str,
                            &coerced,
                            peel_sig.as_ref(),
                            i,
                            &gen.emitted_rust_ref_formals,
                            &gen.current_function_params,
                        );
                    if let Some(ref sig) = peel_sig {
                        coerced = crate::codegen::rust::call_site_borrow::reconcile_explicit_user_clone_into_owned_vec_formal(
                            gen,
                            arg,
                            coerced,
                            sig,
                            i,
                        );
                    }
                    if let Some(ref sig) = peel_sig {
                        gen.peel_fn_trait_or_closure_call_arg(
                            &mut coerced,
                            arg,
                            func_name,
                            sig,
                            i,
                        );
                    }
                    if let Expression::Identifier { name, .. } = arg {
                        if let Some(cloned) = peel_sig.as_ref().and_then(|sig| {
                            crate::codegen::rust::call_site_borrow::clone_reused_binding_for_owned_vec_formal(
                                gen,
                                sig,
                                i,
                                arg,
                                &coerced,
                                Some(func_name),
                            )
                        }) {
                            coerced = cloned;
                        } else {
                        let callee_owned = peel_sig.as_ref().is_some_and(|sig| {
                            let pidx = sig.arg_param_index(i);
                            let emits_shared =
                                crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                    sig, pidx,
                                ) || sig
                                    .emitted_rust_ref_params
                                    .as_ref()
                                    .and_then(|f| f.get(pidx))
                                    .copied()
                                    == Some(true);
                            !emits_shared
                                && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    sig, pidx,
                                ) || gen.preregistered_free_call_arg_emits_owned(func_name, i))
                                && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                                    &gen.signature_registry,
                                    func_name,
                                    Some(sig),
                                    i,
                                )
                        });
                        // P3.390: demoted caller `&Vec` must not be re-cloned into shared-ref callees.
                        let demoted_caller = gen.emitted_rust_ref_formals.contains(name)
                            || gen.caller_formal_emitted_shared_ref(name);
                        if callee_owned
                            && !demoted_caller
                            && !gen.callee_arg_expects_borrow_at_call(func_name, i)
                            && gen.local_binding_reused_after_current_statement(name)
                            && !coerced.ends_with(".clone()")
                        {
                            let base = if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                                coerced.trim_start_matches('&')
                            } else {
                                coerced.as_str()
                            };
                            coerced =
                                gen.append_clone_for_owned_non_copy_binding(name, base);
                        }
                        }
                    } else if matches!(arg, Expression::FieldAccess { .. } | Expression::Index { .. })
                        && !coerced.ends_with(".clone()")
                        && peel_sig.as_ref().is_some_and(|sig| {
                            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                sig,
                                sig.arg_param_index(i),
                            ) && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                                &gen.signature_registry,
                                func_name,
                                Some(sig),
                                i,
                            )
                        })
                    {
                        coerced = gen.maybe_auto_clone_expr_path(arg, &coerced, Some(func_name), Some(i));
                    }
                    if let Expression::Identifier { name, .. } = arg {
                        let lookup = gen.signature_lookup_callee_name(func_name);
                        let candidates = [
                            peel_sig.clone(),
                            signature.clone(),
                            gen.get_signature_with_global(func_name).cloned(),
                            gen.get_signature_with_global(lookup.as_ref()).cloned(),
                            gen.preregistered_free_call_arg_expects_borrow(func_name, i)
                                .then(|| gen.get_signature_with_global(func_name).cloned())
                                .flatten(),
                        ];
                        // Pick first — `strings::join` in the candidate set must not mark
                        // a user `join` owned slot as `&str` (any-homonym leak).
                        let best_sig = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature(
                            candidates,
                        )
                        .map(|sig| {
                            crate::codegen::rust::signature_promotion::local_user_fn_beats_runtime_std_homonym(
                                &gen.signature_registry,
                                func_name,
                                sig,
                            )
                        });
                        let slot_wants_shared = |sig: &crate::analyzer::FunctionSignature| {
                            let pidx = sig.arg_param_index(i);
                            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                sig, pidx,
                            ) || sig.param_types.get(pidx).is_some_and(|t| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                            }) || sig.formal_param_type(pidx).is_some_and(|t| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                            }) || sig
                                .emitted_rust_ref_params
                                .as_ref()
                                .and_then(|f| f.get(pidx))
                                .copied()
                                == Some(true)
                        };
                        let callee_wants_str = best_sig.as_ref().is_some_and(slot_wants_shared)
                            || gen.preregistered_free_call_arg_expects_borrow(func_name, i);
                        // Defining-module owned `String` emission beats stale stubs that
                        // lack `emitted_rust_ref_params` (WDB-301: do not `&` into owned).
                        let callee_confirmed_owned_string = dep_owned
                            || (!callee_wants_str
                                && !dep_shared
                                && best_sig.as_ref().is_some_and(|sig| {
                                let pidx = sig.arg_param_index(i);
                                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    sig, pidx,
                                ) || (sig
                                    .emitted_rust_ref_params
                                    .as_ref()
                                    .and_then(|f| f.get(pidx))
                                    .copied()
                                    == Some(false)
                                    && crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                                        sig, pidx,
                                    ))
                            }));
                        let caller_owned_text = (gen.current_function_params.iter().any(|p| {
                            p.name == *name
                                && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        }) && !gen.emitted_rust_ref_formals.contains(name)
                            && !gen.str_ref_optimized_params.contains(name))
                            || gen.local_var_types.get(name.as_str()).is_some_and(|t| {
                                crate::codegen::rust::types::is_windjammer_text_type(t)
                            });
                        if callee_wants_str
                            && !callee_confirmed_owned_string
                            && caller_owned_text
                            && !coerced.starts_with('&')
                            && !coerced.starts_with("&mut ")
                        {
                            coerced = format!(
                                "&{}",
                                crate::codegen::rust::expression_utilities::borrow_base_expr(
                                    &coerced,
                                )
                            );
                        }
                        let callee_mut = gen.callee_slot_emits_mut_borrow(func_name, i)
                            || peel_sig.as_ref().is_some_and(|sig| {
                                let pidx = sig.arg_param_index(i);
                                matches!(
                                    sig.param_ownership.get(pidx),
                                    Some(crate::analyzer::OwnershipMode::MutBorrowed),
                                ) || sig.param_types.get(pidx).is_some_and(|t| {
                                    matches!(t, crate::parser::Type::MutableReference(_))
                                })
                            });
                        if gen.caller_emits_mut_ref_formal(name) && callee_mut {
                            coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                &coerced,
                            )
                            .to_string();
                            crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                &mut coerced,
                            );
                        }
                        // Terminal: owned caller `Vec` reused after call into owned
                        // callee formal — `.clone()`, never `&binding` (WDB-281 class).
                        if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                            let base = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                &coerced,
                            );
                            if let Expression::Identifier { name, .. } = arg {
                                let caller_vec = gen.current_function_params.iter().any(|p| {
                                    p.name == *name
                                        && (matches!(&p.type_, crate::parser::Type::Vec(_))
                                            || matches!(
                                                &p.type_,
                                                crate::parser::Type::Parameterized(n, _)
                                                    if n == "Vec"
                                            ))
                                });
                                if base == name.as_str()
                                    && caller_vec
                                    && gen.caller_owned_non_copy_formal(name)
                                    && (gen.local_binding_reused_after_current_statement(name)
                                        || gen.in_if_condition)
                                {
                                    coerced = gen.append_clone_for_owned_non_copy_binding(
                                        name, base,
                                    );
                                }
                            }
                        }
                    }
                    // P3.376: demoted free-fn `&str` formals — strip `String::from("lit")`
                    // left by WJ-owned expected-type / owned-literal paths (list_panels).
                    if matches!(
                        arg,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) {
                        let lookup = gen.signature_lookup_callee_name(func_name);
                        let candidates = [
                            peel_sig.clone(),
                            signature.clone(),
                            gen.get_signature_with_global(func_name).cloned(),
                            gen.get_signature_with_global(lookup.as_ref()).cloned(),
                        ];
                        let wants_str = candidates.iter().flatten().any(|sig| {
                            let pidx = sig.arg_param_index(i);
                            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                sig, pidx,
                            ) || sig
                                .emitted_rust_ref_params
                                .as_ref()
                                .and_then(|f| f.get(pidx))
                                .copied()
                                == Some(true)
                                || sig.param_types.get(pidx).is_some_and(|t| {
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                                })
                                || sig.formal_param_type(pidx).is_some_and(|t| {
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                                })
                        });
                        let confirmed_owned = candidates.iter().flatten().any(|sig| {
                            let pidx = sig.arg_param_index(i);
                            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                sig, pidx,
                            ) || (sig
                                .emitted_rust_ref_params
                                .as_ref()
                                .and_then(|f| f.get(pidx))
                                .copied()
                                == Some(false)
                                && crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                                    sig, pidx,
                                ))
                        }) && !candidates.iter().flatten().any(|sig| {
                            let pidx = sig.arg_param_index(i);
                            sig.emitted_rust_ref_params
                                .as_ref()
                                .and_then(|f| f.get(pidx))
                                .copied()
                                == Some(true)
                        });
                        // Only strip owned producers for confirmed shared-ref formals
                        // (WDB-301: do not undo `.to_string()` into owned `String`).
                        if wants_str && !confirmed_owned {
                            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                                arg,
                                &mut coerced,
                            );
                        }
                    }
                    if let Expression::Identifier { name, .. } = arg {
                        // Import aliases (`use owned_pkg::get as query_get`): never look up
                        // the alias string — it collides with foreign `query_get` metadata.
                        if let Some(rs) = gen.get_signature_with_global(func_name) {
                            let pidx = rs.arg_param_index(i);
                            // Emission-confirmed shared ref only — stale param_types
                            // Reference(str) must not force `&` into owned String formals.
                            if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                rs, pidx,
                            ) && gen.caller_owned_non_copy_formal(name)
                                && !coerced.starts_with('&')
                                && !coerced.starts_with("&mut ")
                            {
                                coerced = format!("&{name}");
                            }
                        }
                        // Always honor caller emit-truth: `run(csr: &DenseCsr)` → `touch(csr)`.
                        // Must not sit behind callee registry lookup (WDB-217).
                        if gen.caller_formal_emitted_shared_ref(name)
                            && coerced == format!("&{name}")
                        {
                            coerced = name.to_string();
                        }
                    }
                    if let Expression::Identifier { name, .. } = arg {
                        let mut tmp = coerced.clone();
                        if crate::codegen::rust::string_utilities::rewrite_borrowed_str_clone_to_to_string(
                            &mut tmp,
                            arg,
                            &gen.emitted_rust_ref_formals,
                            &gen.current_function_params,
                        ) {
                            coerced = tmp;
                        }
                    }
                    // Codegen-confirmed `&T` formals: encode Borrow (peel stale `&x.clone()`).
                    if gen.preregistered_free_call_arg_expects_borrow(func_name, i)
                        || gen.callee_arg_expects_borrow_at_call(func_name, i)
                        || (coerced.starts_with('&')
                            && !coerced.starts_with("&mut ")
                            && coerced.ends_with(".clone()"))
                    {
                        coerced = crate::ir::target_encodings::rust_shared_borrow(&coerced);
                    }
                    // Absolute terminal: never emit `n as usize.clone()` (WDB-300).
                    coerced =
                        crate::codegen::rust::expression_utilities::sanitize_cast_trailing_clone(
                            &coerced,
                        );
                    return vec![coerced];
                }
                debug_assert!(
                    false,
                    "IR call-site coercion must be total for non-extern callees when call_sites is on ({func_name})"
                );
                // Phase 5: never fall through to legacy ownership when call_sites is on.
                // IR is total for known callees — emit prepared arg as-is rather than
                // re-entering ~1.5k LOC of pre-IR heuristics.
                return vec![
                    crate::codegen::rust::expression_utilities::sanitize_cast_trailing_clone(
                        &arg_str,
                    ),
                ];
            }

            vec![
                crate::codegen::rust::expression_utilities::sanitize_cast_trailing_clone(&arg_str),
            ]
        })
        .collect()
}
