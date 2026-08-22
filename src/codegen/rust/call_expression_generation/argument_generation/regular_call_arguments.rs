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
            let scope = gen.arg_gen_scope();
            let mut arg_str = gen.generate_expression(arg);
            gen.restore_arg_gen_scope(scope);
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
                        crate::codegen::rust::signature_promotion::refresh_call_site_signature_for_arg(
                            initial_sig.or_else(|| signature.clone()),
                            func_name,
                            i,
                            gen.global_signature_registry.as_deref(),
                            &gen.signature_registry,
                        )
                        .or_else(|| signature.clone())
                    } else {
                        crate::codegen::rust::signature_promotion::refresh_call_site_signature_for_arg(
                            signature.clone(),
                            func_name,
                            i,
                            gen.global_signature_registry.as_deref(),
                            &gen.signature_registry,
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
                    let peel_sig = post_ir_borrow_sig.as_ref().or(signature.as_ref()).or_else(|| {
                        gen.signature_registry.get_signature(func_name).or_else(|| {
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name))
                        })
                    }).or_else(|| {
                        gen.signature_registry.get_signature(simple).or_else(|| {
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(simple))
                        })
                    });
                    if let Some(sig) = peel_sig {
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
                    // Terminal guard when reconcile was skipped (missing peel_sig).
                    gen.peel_stacked_amp_on_emitted_ref_binding(
                        &mut coerced,
                        arg,
                        peel_sig,
                        i,
                        true,
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
                return vec![arg_str];
            }

            vec![arg_str]
        })
        .collect()
}
