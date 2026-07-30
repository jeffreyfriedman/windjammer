//! Plain function call argument lowering (ownership, FFI, casts).

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::super::{expression_helpers, CodeGenerator};

#[allow(clippy::too_many_lines)]
pub(in crate::codegen::rust) fn collect_regular_function_arguments<'ast>(
    gen: &mut CodeGenerator<'ast>,
    func_name: &str,
    _func_str: &str,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
    signature: &Option<crate::analyzer::FunctionSignature>,
    signature_from_simple_fallback: bool,
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
            let is_copy_literal = matches!(
                arg,
                Expression::Literal {
                    value: Literal::Int(_)
                        | Literal::IntSuffixed(_, _)
                        | Literal::Float(_)
                        | Literal::Bool(_),
                    ..
                }
            );

            // Pre-compute ownership collision for the whole argument.
            let has_ownership_collision =
                crate::codegen::rust::call_signature_resolution::has_ownership_collision_for_call(
                    gen, func_name,
                );

            // TDD FIX: Cast int arguments to usize for stdlib methods
            // Vec::with_capacity(size) where size: int → Vec::with_capacity(size as usize)
            // Vec::with_capacity(10) where 10: int literal → Vec::with_capacity(10_usize)
            let method_part = func_name.rsplit("::").next().unwrap_or(func_name);
            if i == 0 && matches!(method_part, "with_capacity" | "reserve")
            {
                match arg {
                    Expression::Identifier { name, .. } => {
                        let already_usize = gen
                            .current_function_params
                            .iter()
                            .find(|p| p.name == *name)
                            .is_some_and(|p| {
                                matches!(&p.type_, Type::Custom(n) if n == "usize")
                            })
                            || gen.local_var_types.get(name).is_some_and(|t| {
                                matches!(t, Type::Custom(n) if n == "usize")
                            });
                        if !already_usize {
                            arg_str = format!("{} as usize", arg_str);
                        }
                    }
                    Expression::Literal {
                        value: Literal::Int(val),
                        ..
                    } => {
                        // Literals: use usize suffix
                        arg_str = format!("{}_usize", val);
                    }
                    _ => {
                        // Other expressions (e.g., calculations): wrap in (expr) as usize
                        if !arg_str.ends_with("_usize") && !arg_str.contains(" as usize") {
                            arg_str = format!("({}) as usize", arg_str);
                        }
                    }
                }
            }

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
                            // fields out from behind `&T` (WDB-043 / E0507).
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
                    // field/index moves from behind references (WDB-043).
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
                    has_ownership_collision,
                    associated_receiver.as_deref(),
                    Some(arguments.len()),
                ) {
                    // IR coercion can miss `&` when registry metadata lags; honor resolved signature.
                    let post_ir_borrow_sig = associated_receiver
                        .as_ref()
                        .and_then(|rt| {
                            func_name
                                .rsplit_once("::")
                                .and_then(|(_, method)| {
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
                        })
                        .or_else(|| {
                            // Free calls: prefer defining-module refreshed `&str` over a
                            // stale call-site stub (WDB-049 replay_to_lsn).
                            let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                            let mut refreshed = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                                gen.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(func_name).cloned()),
                                gen.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(simple).cloned()),
                                gen.signature_registry.get_signature(func_name).cloned(),
                                gen.signature_registry.get_signature(simple).cloned(),
                                signature.clone(),
                            ]);
                            let pidx = refreshed
                                .as_ref()
                                .map(|s| s.arg_param_index(i))
                                .unwrap_or(i);
                            for challenger in [
                                gen.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(func_name)),
                                gen.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(simple)),
                                gen.signature_registry.get_signature(func_name),
                                gen.signature_registry.get_signature(simple),
                            ] {
                                refreshed = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                                    refreshed, challenger, pidx,
                                );
                            }
                            refreshed.or_else(|| signature.clone())
                        });
                    if let Some(ref sig) = post_ir_borrow_sig {
                        if !has_ownership_collision
                            || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                sig,
                                sig.arg_param_index(i),
                            )
                        {
                            let param_idx = sig.arg_param_index(i);
                            let skip_post_ir_stale_borrow = !func_name.contains("::")
                                && crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn_with_global(
                                    &gen.signature_registry,
                                    gen.global_signature_registry.as_deref(),
                                    func_name,
                                    sig,
                                    param_idx,
                                    i,
                                );
                            if !skip_post_ir_stale_borrow
                                && !expression_helpers::is_reference_expression(arg)
                            {
                            let arg_already_rust_ref = matches!(
                                arg,
                                Expression::Identifier { name, .. }
                                    if gen.identifier_already_ref(name)
                                        || gen.str_ref_optimized_params.contains(name.as_str())
                                        || gen.inferred_borrowed_params.contains(name)
                            );
                            let method = func_name.rsplit("::").next().unwrap_or(func_name);
                            let formal_is_copy = sig.param_type_for_arg(i).is_some_and(|t| {
                                gen.is_type_copy(t)
                            });
                            let decision =
                                crate::codegen::rust::call_site_borrow::should_borrow_at_call_site_with_copy_check(
                                    sig,
                                    i,
                                    arg,
                                    &coerced,
                                    method,
                                    arg_already_rust_ref,
                                    None,
                                    formal_is_copy,
                                );
                            crate::codegen::rust::call_site_borrow::apply_call_site_borrow(
                                &decision,
                                &mut coerced,
                            );
                            }
                            let idx = sig.arg_param_index(i);
                            let needs_mut_from_sig = (!crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                sig, idx,
                            ) && sig.param_types.get(idx).is_some_and(|t| {
                                matches!(t, Type::MutableReference(_))
                            })) || matches!(
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                    sig, i,
                                ),
                                OwnershipMode::MutBorrowed,
                            );
                            if needs_mut_from_sig
                                && !coerced.starts_with("&mut ")
                                && matches!(arg, Expression::Identifier { .. })
                            {
                                crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                                    arg,
                                    &mut coerced,
                                    &gen.current_function_params,
                                    &gen.inferred_mut_borrowed_params,
                                );
                            }
                            if skip_post_ir_stale_borrow
                                && (coerced.starts_with("&mut ")
                                    || (coerced.starts_with('&') && !coerced.starts_with("&mut ")))
                            {
                                coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                    &coerced,
                                )
                                .to_string();
                            }
                        }
                    } else if let Some(sig) = gen
                        .signature_registry
                        .get_signature(func_name)
                        .or_else(|| {
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name))
                        })
                    {
                        if !has_ownership_collision {
                            let idx = sig.arg_param_index(i);
                            let needs_mut_from_sig = (!crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                sig, idx,
                            ) && sig.param_types.get(idx).is_some_and(|t| {
                                matches!(t, Type::MutableReference(_))
                            })) || matches!(
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                    sig, i,
                                ),
                                OwnershipMode::MutBorrowed,
                            );
                            if needs_mut_from_sig
                                && !coerced.starts_with("&mut ")
                                && matches!(arg, Expression::Identifier { .. })
                            {
                                crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                                    arg,
                                    &mut coerced,
                                    &gen.current_function_params,
                                    &gen.inferred_mut_borrowed_params,
                                );
                            }
                        }
                    }
                    let callee_wants_mut_arg = {
                        let refreshed = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                            signature.clone(),
                            gen.signature_registry.get_signature(func_name).cloned(),
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name).cloned()),
                            func_name
                                .rsplit("::")
                                .next()
                                .and_then(|simple| {
                                    gen.signature_registry.get_signature(simple).cloned().or_else(
                                        || {
                                            gen.global_signature_registry.as_ref().and_then(|g| {
                                                g.get_signature(simple).cloned()
                                            })
                                        },
                                    )
                                }),
                        ]);
                        let owned_formal = refreshed.as_ref().is_some_and(|sig| {
                            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                sig,
                                sig.arg_param_index(i),
                            )
                        });
                        if owned_formal {
                            false
                        } else {
                            gen.function_emitted_mut_arg_indices
                                .get(func_name)
                                .or_else(|| {
                                    func_name.rsplit("::").next().and_then(|simple| {
                                        gen.function_emitted_mut_arg_indices.get(simple)
                                    })
                                })
                                .is_some_and(|indices| indices.contains(&i))
                                || signature.as_ref().is_some_and(|sig| {
                                    crate::codegen::rust::call_signature_resolution::callee_user_arg_expects_mut_borrow(
                                        sig, i,
                                    )
                                })
                                || gen
                                    .signature_registry
                                    .get_signature(func_name)
                                    .or_else(|| {
                                        gen.global_signature_registry
                                            .as_ref()
                                            .and_then(|g| g.get_signature(func_name))
                                    })
                                    .is_some_and(|sig| {
                                        crate::codegen::rust::call_signature_resolution::callee_user_arg_expects_mut_borrow(
                                            sig, i,
                                        )
                                    })
                        }
                    };
                    if (!has_ownership_collision || callee_wants_mut_arg)
                        && callee_wants_mut_arg
                        && !coerced.starts_with("&mut ")
                        && crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(
                            arg,
                        )
                    {
                        crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                            arg,
                            &mut coerced,
                            &gen.current_function_params,
                            &gen.inferred_mut_borrowed_params,
                        );
                    }
                    // Owned callee formals (codegen refresh): strip post-IR `&` / `&mut`
                    // that stale MutBorrowed stubs re-applied (dogfood).
                    if let Some(sig) = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                        signature.clone(),
                        gen.signature_registry.get_signature(func_name).cloned(),
                        gen.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(func_name).cloned()),
                        func_name.rsplit("::").next().and_then(|simple| {
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(simple).cloned())
                                .or_else(|| gen.signature_registry.get_signature(simple).cloned())
                        }),
                    ]) {
                        let pidx = sig.arg_param_index(i);
                        let shared_ref = crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            &sig, pidx,
                        ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(&sig, pidx);
                        if !shared_ref
                            && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                &sig, pidx,
                            ) || crate::ir::signature_bridge::call_site_expects_owned_pass(
                                &sig, pidx,
                            ))
                            && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                                &gen.signature_registry,
                                func_name,
                                Some(&sig),
                                i,
                            )
                        {
                            if coerced.starts_with("&mut ")
                                || (coerced.starts_with('&') && !coerced.starts_with("&mut "))
                            {
                                coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                    &coerced,
                                )
                                .to_string();
                            }
                        }
                    }
                    if coerced.starts_with("&mut ") {
                        if let Some(sig) = signature.as_ref().or_else(|| {
                            gen.signature_registry.get_signature(func_name).or_else(|| {
                                gen.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(func_name))
                            })
                        }) {
                            let idx = sig.arg_param_index(i);
                            if let Some(formal) = sig.formal_param_type(idx) {
                                let bare = match formal {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                if crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                                    && !matches!(
                                        formal,
                                        Type::Reference(_) | Type::MutableReference(_)
                                    )
                                {
                                    coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                        &coerced,
                                    )
                                    .to_string();
                                }
                            }
                        }
                    }
                    if matches!(
                        arg,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) && !coerced.ends_with(".to_string()")
                    {
                        // Same guards as `apply_owned_string_literal_coercion`: do not
                        // invent `.to_string()` for unresolved inline/user-module paths.
                        let skip_lit_owned = gen.inline_module_qualified_call(func_name)
                            || crate::codegen::rust::call_signature_resolution::is_lowercase_user_module_qualified_call(
                                func_name,
                            );
                        if !skip_lit_owned {
                        // Prefer defining-module / post-IR refreshed contracts over the
                        // analyzer call-site stub (Phase-2 `&str` emission must win).
                        let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                        let allow_simple = !crate::codegen::rust::call_signature_resolution::is_lowercase_user_module_qualified_call(
                            func_name,
                        );
                        let mut sig_for_lit = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                            post_ir_borrow_sig.clone(),
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name).cloned()),
                            if allow_simple {
                                gen.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(simple).cloned())
                            } else {
                                None
                            },
                            gen.signature_registry.get_signature(func_name).cloned(),
                            if allow_simple {
                                gen.signature_registry.get_signature(simple).cloned()
                            } else {
                                None
                            },
                            signature.clone(),
                        ]);
                        let lit_pidx = sig_for_lit
                            .as_ref()
                            .map(|s| s.arg_param_index(i))
                            .unwrap_or(i);
                        for challenger in [
                            post_ir_borrow_sig.as_ref(),
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name)),
                            if allow_simple {
                                gen.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(simple))
                            } else {
                                None
                            },
                            gen.signature_registry.get_signature(func_name),
                            if allow_simple {
                                gen.signature_registry.get_signature(simple)
                            } else {
                                None
                            },
                        ] {
                            sig_for_lit = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                                sig_for_lit, challenger, lit_pidx,
                            );
                        }
                        if crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion_with_enum(
                            sig_for_lit.as_ref(),
                            i,
                            func_name.rsplit("::").next(),
                            func_name
                                .split("::")
                                .next()
                                .filter(|q| q.chars().next().is_some_and(|c| c.is_ascii_uppercase())),
                            Some(&gen.enum_variant_types),
                            func_name.split("::").next(),
                        ) {
                            coerced = format!(
                                "{}.to_string()",
                                coerced.trim_start_matches('&')
                            );
                        }
                        }
                    }
                    if !coerced.starts_with('&')
                        && matches!(
                            arg,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        )
                        && !matches!(
                            arg,
                            Expression::Identifier { name, .. }
                                if gen.binding_emits_as_rust_shared_ref(name)
                        )
                    {
                        let module = func_name.split("::").next().unwrap_or("");
                        let method = func_name.rsplit("::").next().unwrap_or(func_name);
                        if crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                            &gen.signature_registry,
                            func_name,
                            post_ir_borrow_sig.as_ref(),
                            i,
                        ) {
                            coerced = format!("&{coerced}");
                        }
                    }
                    if matches!(arg, Expression::FieldAccess { .. } | Expression::Index { .. })
                        && !coerced.starts_with('&')
                        && !coerced.ends_with(".clone()")
                    {
                        if post_ir_borrow_sig.as_ref().is_some_and(|sig| {
                            let idx = sig.arg_param_index(i);
                            let ownership =
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                    sig, i,
                                );
                            let wants_borrow = matches!(
                                ownership,
                                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                            ) || sig.param_types.get(idx).is_some_and(|t| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                            }) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                sig, idx,
                            );
                            let wants_owned = matches!(ownership, OwnershipMode::Owned)
                                && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    sig, idx,
                                );
                            wants_borrow
                                && !wants_owned
                                && (sig.formal_param_type(idx).is_some_and(
                                    crate::codegen::rust::types::is_windjammer_text_type,
                                ) || sig.param_types.get(idx).is_some_and(|t| {
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                                        || crate::codegen::rust::types::is_windjammer_text_type(t)
                                }))
                        }) {
                            coerced = format!("&{coerced}");
                        }
                    }
                    if let (Some(rt), Some(method)) = (
                        associated_receiver.as_deref(),
                        func_name.rsplit_once("::").map(|(_, m)| m),
                    ) {
                        let sig_for_vec = post_ir_borrow_sig
                            .clone()
                            .or_else(|| {
                                gen.resolve_method_function_signature(
                                    rt,
                                    method,
                                    arguments.len(),
                                )
                            })
                            .unwrap_or_default();
                        coerced = crate::codegen::rust::call_site_borrow::maybe_borrow_owned_vec_local_for_ref_formal(
                            gen,
                            &sig_for_vec,
                            i,
                            arg,
                            coerced,
                            Some(rt),
                            Some(method),
                            Some(arguments.len()),
                        );
                    }
                    if let Some(ref sig) = post_ir_borrow_sig {
                        let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                        let pidx = sig.arg_param_index(i);
                        // Prefer defining-module refresh with shared-ref slots over importer
                        // stubs that recorded `emitted_rust_ref_params = Some([false, …])`.
                        let mut enforce_sig = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name).cloned()),
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(simple).cloned()),
                            Some(sig.clone()),
                            gen.signature_registry.get_signature(func_name).cloned(),
                            gen.signature_registry.get_signature(simple).cloned(),
                        ])
                        .unwrap_or_else(|| sig.clone());
                        enforce_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                            Some(enforce_sig),
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name))
                                .or_else(|| {
                                    gen.global_signature_registry
                                        .as_ref()
                                        .and_then(|g| g.get_signature(simple))
                                }),
                            pidx,
                        )
                        .unwrap_or_else(|| sig.clone());
                        let pidx = enforce_sig.arg_param_index(i);
                        // Don't let a stale analyzer stub strip IR's confirmed `&item`
                        // after collision-aware IR already lowered the arg (bug_e0308).
                        // Also keep `&` when *any* registry view emits shared-ref for this slot.
                        let keep_shared_ref = coerced.starts_with('&')
                            && (crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                &enforce_sig, pidx,
                            ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(
                                &enforce_sig, pidx,
                            ) || gen
                                .global_signature_registry
                                .as_ref()
                                .and_then(|g| {
                                    g.get_signature(func_name)
                                        .or_else(|| g.get_signature(simple))
                                })
                                .is_some_and(|gs| {
                                    let gp = gs.arg_param_index(i);
                                    crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                        gs, gp,
                                    )
                                }));
                        if !keep_shared_ref {
                            gen.enforce_call_site_ownership_contract(
                                &mut coerced,
                                arg,
                                &enforce_sig,
                                pidx,
                                func_name,
                                i,
                            );
                        }
                    }
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
                    let callee_wants_mut = {
                        let refreshed = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                            post_ir_borrow_sig.clone(),
                            signature.clone(),
                            gen.signature_registry.get_signature(func_name).cloned(),
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name).cloned()),
                            func_name.rsplit("::").next().and_then(|simple| {
                                gen.global_signature_registry
                                    .as_ref()
                                    .and_then(|g| g.get_signature(simple).cloned())
                                    .or_else(|| gen.signature_registry.get_signature(simple).cloned())
                            }),
                        ]);
                        let owned_formal = refreshed.as_ref().is_some_and(|sig| {
                            let pidx = sig.arg_param_index(i);
                            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                sig, pidx,
                            ) || crate::ir::signature_bridge::call_site_expects_owned_pass(sig, pidx)
                        });
                        if owned_formal {
                            false
                        } else {
                            gen.function_emitted_mut_arg_indices
                                .get(func_name)
                                .or_else(|| {
                                    func_name.rsplit("::").next().and_then(|simple| {
                                        gen.function_emitted_mut_arg_indices.get(simple)
                                    })
                                })
                                .is_some_and(|indices| indices.contains(&i))
                                || post_ir_borrow_sig.as_ref().is_some_and(|sig| {
                                    crate::codegen::rust::call_signature_resolution::callee_user_arg_expects_mut_borrow(
                                        sig, i,
                                    )
                                })
                                || gen
                                    .signature_registry
                                    .get_signature(func_name)
                                    .or_else(|| {
                                        gen.global_signature_registry
                                            .as_ref()
                                            .and_then(|g| g.get_signature(func_name))
                                    })
                                    .is_some_and(|sig| {
                                        crate::codegen::rust::call_signature_resolution::callee_user_arg_expects_mut_borrow(
                                            sig, i,
                                        )
                                    })
                        }
                    };
                    if callee_wants_mut
                        && !coerced.starts_with("&mut ")
                        && crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(
                            arg,
                        )
                    {
                        crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                            arg,
                            &mut coerced,
                            &gen.current_function_params,
                            &gen.inferred_mut_borrowed_params,
                        );
                    }
                    // Final owned-contract peel after late mut-arg re-application.
                    // Never peel confirmed `&str` / shared-ref formals (dogfood).
                    if let Some(sig) = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                        post_ir_borrow_sig.clone(),
                        signature.clone(),
                        gen.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(func_name).cloned()),
                        gen.signature_registry.get_signature(func_name).cloned(),
                        func_name.rsplit("::").next().and_then(|simple| {
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(simple).cloned())
                                .or_else(|| gen.signature_registry.get_signature(simple).cloned())
                        }),
                    ]) {
                        let pidx = sig.arg_param_index(i);
                        let shared_ref = crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            &sig, pidx,
                        ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(&sig, pidx);
                        if !shared_ref
                            && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                &sig, pidx,
                            ) || crate::ir::signature_bridge::call_site_expects_owned_pass(
                                &sig, pidx,
                            ))
                            && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                                &gen.signature_registry,
                                func_name,
                                Some(&sig),
                                i,
                            )
                        {
                            if coerced.starts_with("&mut ")
                                || (coerced.starts_with('&') && !coerced.starts_with("&mut "))
                            {
                                coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                    &coerced,
                                )
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
                    // IR early-return skips the non-IR finalize path — still align text
                    // FieldAccess args with codegen-refreshed `&str` formals (WDB-049
                    // `replay_to_lsn(self.path)` vs `replay_all(&self.path)`).
                    // Run even under type-collision (String→&str refresh homonym): collision
                    // must not block text FieldAccess borrow for confirmed `&str` formals.
                    {
                        let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                        // Prefer defining-module / global refresh before call-site stubs so
                        // cross-file `replay_to_lsn(path: &str)` beats a stale local
                        // `emitted_rust_ref_params = Some([false, …])` (WDB-049).
                        let mut text_sig = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name).cloned()),
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(simple).cloned()),
                            gen.signature_registry.get_signature(func_name).cloned(),
                            gen.signature_registry.get_signature(simple).cloned(),
                            post_ir_borrow_sig.clone(),
                            signature.clone(),
                        ]);
                        let pidx_for_upgrade = text_sig
                            .as_ref()
                            .map(|s| s.arg_param_index(i))
                            .unwrap_or(i);
                        for challenger in [
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name)),
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(simple)),
                            gen.signature_registry.get_signature(func_name),
                            gen.signature_registry.get_signature(simple),
                        ] {
                            text_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                                text_sig,
                                challenger,
                                pidx_for_upgrade,
                            );
                        }
                        let arg_already_rust_ref = matches!(
                            arg,
                            Expression::Identifier { name, .. }
                                if gen.identifier_already_ref(name)
                                    || gen.str_ref_optimized_params.contains(name.as_str())
                                    || gen.inferred_borrowed_params.contains(name)
                        );
                        crate::codegen::rust::string_utilities::finalize_borrowed_text_call_site_arg(
                            text_sig.as_ref(),
                            i,
                            None,
                            arg,
                            &mut coerced,
                            arg_already_rust_ref,
                        );
                        if matches!(arg, Expression::FieldAccess { .. } | Expression::Index { .. })
                        {
                            let pidx = text_sig
                                .as_ref()
                                .map(|s| s.arg_param_index(i))
                                .unwrap_or(i);
                            coerced = gen.ensure_ref_for_owned_string_field_when_callee_expects_str(
                                &text_sig,
                                pidx,
                                arg,
                                coerced,
                                false,
                            );
                            // Belt-and-suspenders: confirmed `&str` formal + owned String
                            // FieldAccess must borrow (WDB-049 replay_to_lsn(self.path)).
                            if !coerced.starts_with('&')
                                && text_sig.as_ref().is_some_and(|sig| {
                                    crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                        sig, pidx,
                                    ) && (crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                                        sig, pidx,
                                    ) || sig.param_types.get(pidx).is_some_and(|t| {
                                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                                            || matches!(
                                                t,
                                                Type::Reference(inner)
                                                    if crate::codegen::rust::types::is_windjammer_text_type(inner)
                                            )
                                    }))
                                })
                                && gen.infer_expression_type(arg).as_ref().is_some_and(
                                    crate::codegen::rust::types::is_windjammer_text_type,
                                )
                            {
                                coerced = format!("&{coerced}");
                            }
                        }
                    }
                    // Final owned peel immediately before IR early-return.
                    // Prefer *any* candidate that confirms owned (defining-module refresh), not
                    // merely the first `emitted_rust_ref_params` with a text `&str` slot —
                    // stale importer stubs can win `pick_codegen_refreshed` via unrelated `true`
                    // flags while param 0 still carries MutableReference (dogfood).
                    // But never peel when *any* candidate confirms a shared-ref formal for this
                    // slot (dogfood)` / `&str` formals).
                    {
                        let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                        let owned_candidates = [
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(func_name)),
                            gen.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(simple)),
                            gen.signature_registry.get_signature(func_name),
                            gen.signature_registry.get_signature(simple),
                            post_ir_borrow_sig.as_ref(),
                            signature.as_ref(),
                        ];
                        let keep_shared_ref = owned_candidates.iter().flatten().any(|sig| {
                            let pidx = sig.arg_param_index(i);
                            crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                sig, pidx,
                            ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(
                                sig, pidx,
                            )
                        });
                        let peel_owned = !keep_shared_ref
                            && owned_candidates.iter().flatten().any(|sig| {
                                let pidx = sig.arg_param_index(i);
                                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    sig, pidx,
                                ) || crate::ir::signature_bridge::call_site_expects_owned_pass(
                                    sig, pidx,
                                )
                            })
                            // Runtime-scanner borrow (json::get, subprocess::spawn) must
                            // survive even when a layered WJ stub claims owned.
                            && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                                &gen.signature_registry,
                                func_name,
                                post_ir_borrow_sig.as_ref().or(signature.as_ref()),
                                i,
                            );
                        if peel_owned
                            && (coerced.starts_with("&mut ")
                                || (coerced.starts_with('&') && !coerced.starts_with("&mut ")))
                        {
                            coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                &coerced,
                            )
                            .to_string();
                        }
                    }
                    return vec![coerced];
                }
                let callee_sig = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    signature.clone(),
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(func_name).cloned()),
                    gen.signature_registry.get_signature(func_name).cloned(),
                    func_name.rsplit("::").next().and_then(|simple| {
                        gen.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(simple).cloned())
                            .or_else(|| gen.signature_registry.get_signature(simple).cloned())
                    }),
                ]);
                if let Some(sig) = callee_sig.as_ref() {
                    let pidx = sig.arg_param_index(i);
                    let owned_formal =
                        crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig, pidx,
                        ) || crate::ir::signature_bridge::call_site_expects_owned_pass(sig, pidx);
                    let needs_mut = !owned_formal
                        && (matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                sig, i,
                            ),
                            OwnershipMode::MutBorrowed,
                        ) || sig.param_types.get(pidx).is_some_and(|t| {
                            matches!(t, Type::MutableReference(_))
                        }));
                    if !has_ownership_collision
                        && needs_mut
                        && crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(arg)
                    {
                        let mut coerced = arg_str.clone();
                        if !coerced.starts_with("&mut ") {
                            crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                                arg,
                                &mut coerced,
                                &gen.current_function_params,
                                &gen.inferred_mut_borrowed_params,
                            );
                        }
                        return vec![coerced];
                    }
                }
                arg_str = gen.maybe_auto_clone_call_arg(arg, &arg_str, Some(func_name), Some(i));
                if !arg_str.starts_with('&')
                    && matches!(arg, Expression::Identifier { .. })
                    && crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                        func_name,
                    )
                {
                    if let (Some(rt), Some(method)) = (
                        associated_receiver.as_deref(),
                        func_name.rsplit_once("::").map(|(_, m)| m),
                    ) {
                        if gen.method_registry_arg_expects_shared_borrow(
                            rt,
                            method,
                            i,
                            arguments.len(),
                        ) {
                            arg_str = format!("&{arg_str}");
                        }
                    }
                }
            }

            // Auto-convert string literals to String for functions expecting owned String
            // THE WINDJAMMER WAY: Smart inference based on available information!
            if matches!(
                arg,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            ) {
                let skip_owned_string_literal = {
                    let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                    let refreshed = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                        gen.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(func_name).cloned()),
                        gen.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(simple).cloned()),
                        gen.signature_registry.get_signature(func_name).cloned(),
                        gen.signature_registry.get_signature(simple).cloned(),
                        signature.clone(),
                    ]);
                    let mut text_sig = refreshed.or_else(|| signature.clone());
                    let pidx_for_upgrade = text_sig
                        .as_ref()
                        .map(|s| s.arg_param_index(i))
                        .unwrap_or(i);
                    for challenger in [
                        gen.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(func_name)),
                        gen.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(simple)),
                        gen.signature_registry.get_signature(func_name),
                        gen.signature_registry.get_signature(simple),
                    ] {
                        text_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                            text_sig, challenger, pidx_for_upgrade,
                        );
                    }
                    text_sig.as_ref().is_some_and(|sig| {
                        let pidx = sig.arg_param_index(i);
                        crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                            sig, i,
                        ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            sig, pidx,
                        )
                    })
                };

                let should_convert = if skip_owned_string_literal {
                    false
                } else if let Some(ref sig) = signature {
                    let method = func_name.rsplit("::").next();
                    let is_constructor = func_name == "new" || func_name.ends_with("::new");
                    if sig.is_extern {
                        sig.param_types.get(i).is_some_and(|ty| {
                            crate::codegen::rust::string_utilities::param_is_owned_string_type(ty)
                        })
                    } else if is_constructor
                        && sig.param_types.get(i).is_some_and(
                            crate::codegen::rust::types::is_windjammer_text_type,
                        )
                    {
                        // Constructor string params use struct-literal storage in the
                        // callee body — literals are passed as &str at the call site.
                        false
                    } else if signature_from_simple_fallback && {
                        let qualifier = func_name.split("::").next().unwrap_or("");
                        qualifier.chars().next().is_some_and(|c| c.is_lowercase())
                    } {
                        // Fallback-resolved from module::function: the signature may
                        // be from a different module. Don't trust ownership for
                        // string coercion — the actual target may take &str.
                        false
                    } else if gen.inline_module_qualified_call(func_name) {
                        false
                    } else {
                        crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion_with_enum(
                            Some(sig),
                            i,
                            method,
                            func_name
                                .split("::")
                                .next()
                                .filter(|q| q.chars().next().is_some_and(|c| c.is_ascii_uppercase())),
                            Some(&gen.enum_variant_types),
                            func_name.split("::").next(),
                        )
                    }
                } else {
                    // No signature found — check enum variant registry
                    // WINDJAMMER FIX: Enum variant constructors like GameEvent::ItemPickup("text")
                    // need .to_string() when the variant field is String type
                    if let Some(variant_types) = gen.enum_variant_types.get(func_name) {
                        // TDD FIX: Check for both Type::String and Type::Custom("String")
                        variant_types.get(i).is_some_and(|ty| {
                            matches!(ty, Type::String)
                                || matches!(ty, Type::Custom(name) if name == "String")
                        })
                    } else {
                        // Fallback heuristic for constructors
                        func_name == "new" || func_name.ends_with("::new")
                    }
                };

                if should_convert {
                    arg_str = format!("{}.to_string()", arg_str);
                }
            }

            crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
                signature.as_ref(),
                i,
                func_name.rsplit("::").next(),
                arg,
                &mut arg_str,
                func_name
                    .split("::")
                    .next()
                    .filter(|q| q.chars().next().is_some_and(|c| c.is_ascii_uppercase())),
                Some(&gen.enum_variant_types),
                func_name.split("::").next(),
            );

            if !has_ownership_collision {
                let arg_already_rust_ref = matches!(
                    arg,
                    Expression::Identifier { name, .. }
                        if gen.identifier_already_ref(name)
                            || gen.str_ref_optimized_params.contains(name.as_str())
                            || gen.inferred_borrowed_params.contains(name)
                );
                let mut text_sig = signature.clone();
                if let Some(global) = gen.global_signature_registry.as_ref() {
                    let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                    let refreshed =
                        crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                            text_sig.clone(),
                            global.get_signature(func_name).cloned(),
                            global.get_signature(simple).cloned(),
                            gen.signature_registry.get_signature(func_name).cloned(),
                            gen.signature_registry.get_signature(simple).cloned(),
                        ]);
                    if refreshed.is_some() {
                        text_sig = refreshed;
                    }
                    let pidx_for_upgrade = text_sig
                        .as_ref()
                        .map(|s| s.arg_param_index(i))
                        .unwrap_or(i);
                    for challenger in [
                        global.get_signature(func_name),
                        global.get_signature(simple),
                        gen.signature_registry.get_signature(func_name),
                        gen.signature_registry.get_signature(simple),
                    ] {
                        text_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                            text_sig,
                            challenger,
                            pidx_for_upgrade,
                        );
                    }
                }
                // Re-finalize literals with refreshed `&String` / `&str` param types.
                crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
                    text_sig.as_ref(),
                    i,
                    func_name.rsplit("::").next(),
                    arg,
                    &mut arg_str,
                    func_name
                        .split("::")
                        .next()
                        .filter(|q| q.chars().next().is_some_and(|c| c.is_ascii_uppercase())),
                    Some(&gen.enum_variant_types),
                    func_name.split("::").next(),
                );
                crate::codegen::rust::string_utilities::finalize_borrowed_text_call_site_arg(
                    text_sig.as_ref(),
                    i,
                    func_name
                        .rsplit_once("::")
                        .filter(|(rt, _)| rt.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
                        .map(|(rt, _)| rt),
                    arg,
                    &mut arg_str,
                    arg_already_rust_ref,
                );
                if matches!(arg, Expression::FieldAccess { .. } | Expression::Index { .. }) {
                    let pidx = text_sig
                        .as_ref()
                        .map(|s| s.arg_param_index(i))
                        .unwrap_or(i);
                    arg_str = gen.ensure_ref_for_owned_string_field_when_callee_expects_str(
                        &text_sig,
                        pidx,
                        arg,
                        arg_str,
                        false,
                    );
                }
            } else if matches!(arg, Expression::FieldAccess { .. } | Expression::Index { .. }) {
                // Type collision (e.g. String→&str refresh) must not block text FieldAccess
                // borrows for confirmed `&str` formals (WDB-049 replay_to_lsn).
                let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                let mut text_sig = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(func_name).cloned()),
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned()),
                    gen.signature_registry.get_signature(func_name).cloned(),
                    gen.signature_registry.get_signature(simple).cloned(),
                    signature.clone(),
                ]);
                let pidx_for_upgrade = text_sig
                    .as_ref()
                    .map(|s| s.arg_param_index(i))
                    .unwrap_or(i);
                for challenger in [
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(func_name)),
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple)),
                    gen.signature_registry.get_signature(func_name),
                    gen.signature_registry.get_signature(simple),
                ] {
                    text_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                        text_sig,
                        challenger,
                        pidx_for_upgrade,
                    );
                }
                crate::codegen::rust::string_utilities::finalize_borrowed_text_call_site_arg(
                    text_sig.as_ref(),
                    i,
                    None,
                    arg,
                    &mut arg_str,
                    false,
                );
                let pidx = text_sig
                    .as_ref()
                    .map(|s| s.arg_param_index(i))
                    .unwrap_or(i);
                arg_str = gen.ensure_ref_for_owned_string_field_when_callee_expects_str(
                    &text_sig,
                    pidx,
                    arg,
                    arg_str,
                    false,
                );
            }

            // `const SCOPE_*: string` lowers to &'static str; callee params typed `String` need owned.
            if let Expression::Identifier { name, .. } = arg {
                let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                let mut owned_text_sig = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    signature.clone(),
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(func_name).cloned()),
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned()),
                    gen.signature_registry.get_signature(func_name).cloned(),
                    gen.signature_registry.get_signature(simple).cloned(),
                ]);
                let pidx_for_owned = owned_text_sig
                    .as_ref()
                    .map(|s| s.arg_param_index(i))
                    .unwrap_or(i);
                for challenger in [
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(func_name)),
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple)),
                    gen.signature_registry.get_signature(func_name),
                    gen.signature_registry.get_signature(simple),
                ] {
                    owned_text_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                        owned_text_sig,
                        challenger,
                        pidx_for_owned,
                    );
                }
                let param_wants_owned_string = owned_text_sig.as_ref().is_some_and(|sig| {
                    let pidx = sig.arg_param_index(i);
                    if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        sig, pidx,
                    ) || sig.param_types.get(pidx).is_some_and(|ty| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
                            || matches!(
                                ty,
                                Type::Reference(inner)
                                    if crate::codegen::rust::types::is_windjammer_text_type(inner)
                            )
                    }) {
                        return false;
                    }
                    let mut ownership =
                        if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                            func_name,
                        ) {
                            func_name.rsplit_once("::").map(|(receiver, _)| {
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                    sig,
                                    i,
                                    Some(receiver),
                                )
                            }).unwrap_or_else(|| {
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                    sig, i,
                                )
                            })
                        } else {
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                sig, i,
                            )
                        };
                    if let Some(global) = gen.global_signature_registry() {
                        if let Some(global_own) =
                            crate::codegen::rust::call_signature_resolution::global_suffix_param_ownership(
                                global,
                                func_name,
                                arguments.len(),
                                i,
                            )
                        {
                            if matches!(
                                global_own,
                                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                            ) {
                                ownership = global_own;
                            }
                        }
                    }
                    matches!(ownership, OwnershipMode::Owned)
                        && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig, pidx,
                        )
                        && sig.param_types.get(pidx).is_some_and(|ty| {
                            crate::codegen::rust::string_utilities::param_is_owned_string_type(ty)
                        })
                });
                let is_string_const = crate::codegen::rust::string_utilities::is_string_const_identifier(
                    name,
                    gen.auto_clone_analysis.as_ref(),
                );
                if param_wants_owned_string && !arg_str.ends_with(".to_string()") {
                    let is_text_arg = is_string_const
                        || gen.str_ref_optimized_params.contains(name)
                        || gen.inferred_borrowed_params.contains(name)
                        || gen.current_function_params.iter().any(|p| {
                            p.name == *name
                                && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        });
                    if is_text_arg {
                        arg_str = format!("{}.to_string()", arg_str);
                    }
                }
            }

            if let Some(ref sig) = signature {
                let is_cross_module = func_name.contains("::");
                let all_params_borrowed = !is_extern_call
                    && !sig.is_extern
                    && !(is_cross_module && has_ownership_collision)
                    && !sig.param_ownership.is_empty()
                    && sig
                        .param_ownership
                        .iter()
                        .all(|o| matches!(o, OwnershipMode::Borrowed));
                if all_params_borrowed {
                    let param_idx = sig.arg_param_index(i);
                    let skip_stale_owned_free_fn = !func_name.contains("::")
                        && crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn_with_global(
                            &gen.signature_registry,
                            gen.global_signature_registry.as_deref(),
                            func_name,
                            sig,
                            param_idx,
                            i,
                        );
                    if skip_stale_owned_free_fn {
                        // Registry/formal contract is owned — ignore stale all-Borrowed metadata.
                    } else {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);
                    let already_ref = if let Expression::Identifier { name, .. } = arg {
                        gen.identifier_already_ref(name)
                    } else {
                        arg_str.starts_with('&') && !arg_str.starts_with("&&")
                    };
                    let is_user_closure_param = if let Expression::Identifier { name, .. } = arg {
                        gen.in_user_written_closure && gen.user_closure_params.contains(name)
                    } else {
                        false
                    };
                    let is_copy_literal = is_copy_literal;
                    let param_is_copy_scalar = sig.param_type_for_arg(i).is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(t)
                            && gen.is_type_copy(t)
                            && !crate::codegen::rust::types::is_windjammer_text_type(t)
                    });
                    let arg_is_copy_scalar = gen
                        .infer_expression_type(arg)
                        .as_ref()
                        .is_some_and(|t| {
                            gen.is_type_copy(t)
                                && !crate::codegen::rust::types::is_windjammer_text_type(t)
                        });
                    let is_string_literal = matches!(
                        arg,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    );
                    if !already_ref
                        && !is_user_closure_param
                        && !arg_str.starts_with('&')
                        && !is_copy_literal
                        && !param_is_copy_scalar
                        && !arg_is_copy_scalar
                        && !is_string_literal
                    {
                        crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                            &mut arg_str,
                        );
                        return vec![arg_str];
                    }
                    }
                }
            }

            // Check if this parameter expects a borrow
            // Skip ownership inference for extern function calls - they have explicit types
            if let Some(ref sig) = signature {
                if sig.is_extern {
                    // Auto-convert mut locals to &mut when FFI param is *mut T
                    // This eliminates Rust leakage: users write `ffi_fn(x)` not `ffi_fn(&mut x)`
                    if let Some(param_type) = sig.param_types.get(i) {
                        if matches!(
                            param_type,
                            crate::parser::ast::types::Type::RawPointer { mutable: true, .. }
                        ) {
                            return vec![format!("&mut {}", arg_str)];
                        }
                    }
                    return vec![arg_str];
                }

                let mut ownership =
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, i,
                    );
                if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                    func_name,
                ) {
                    if let Some((receiver, _)) = func_name.rsplit_once("::") {
                        ownership = crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                            sig,
                            i,
                            Some(receiver),
                        );
                    }
                }
                if let Some(formal_ty) = sig.formal_param_type(i) {
                    if !matches!(
                        formal_ty,
                        Type::Reference(_) | Type::MutableReference(_)
                    ) && gen.is_type_copy(formal_ty)
                    {
                        let effective =
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                sig, i,
                            );
                        if !matches!(effective, OwnershipMode::MutBorrowed) {
                            ownership = OwnershipMode::Owned;
                        } else {
                            ownership = OwnershipMode::MutBorrowed;
                        }
                    }
                }
                if matches!(ownership, OwnershipMode::Owned)
                    && !crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                        func_name,
                    )
                {
                    if let Some(global) = gen.global_signature_registry() {
                        if let Some(global_own) =
                            crate::codegen::rust::call_signature_resolution::global_suffix_param_ownership(
                                global,
                                func_name,
                                arguments.len(),
                                i,
                            )
                        {
                            if matches!(global_own, OwnershipMode::Owned) {
                                ownership = OwnershipMode::Owned;
                            }
                        }
                    }
                    if matches!(ownership, OwnershipMode::Owned) {
                        let lookup_name = if let Some(method) = func_name.strip_prefix("Self::") {
                            if gen.in_impl_block {
                                gen.current_struct_name.as_ref().map(|tn| {
                                    format!("{tn}::{method}")
                                })
                            } else {
                                None
                            }
                        } else {
                            Some(func_name.to_string())
                        };
                        if let Some(lookup_name) = lookup_name {
                            let receiver = lookup_name.rsplit("::").next().and_then(|_| {
                                lookup_name
                                    .rsplit_once("::")
                                    .map(|(qual, _)| qual)
                            });
                            if let Some(resolved) = gen.resolve_call_signature_with_global(
                                &lookup_name,
                                receiver,
                                arguments.len(),
                            ) {
                                let upgraded =
                                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                        &resolved.sig, i,
                                    );
                                // Copy types pass by value only when the callee formal is owned.
                                // When the callee expects &mut (MutBorrowed), honor the upgrade.
                                let param_is_copy = resolved
                                    .sig
                                    .formal_param_type(resolved.sig.arg_param_index(i))
                                    .is_some_and(|t| gen.is_type_copy(t));
                                let skip_copy_owned = param_is_copy
                                    && !matches!(upgraded, OwnershipMode::MutBorrowed);
                                if !matches!(upgraded, OwnershipMode::Owned)
                                    && !skip_copy_owned
                                    && (func_name.starts_with("Self::")
                                        || !crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                                            &lookup_name,
                                        )
                                        || matches!(
                                            upgraded,
                                            OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        ))
                                {
                                    ownership = upgraded;
                                }
                            }
                        }
                    }
                }
                if matches!(ownership, OwnershipMode::Owned)
                    && i == 0
                    && crate::codegen::rust::call_signature_resolution::is_external_module_qualified_call(
                        func_name,
                    )
                {
                    if let Expression::Identifier { name, .. } = arg {
                        // Copy caller params (e.g. AppDeps) pass by value even when the
                        // analyzer marked them MutBorrowed for in-place field updates.
                        let caller_param_is_copy = gen
                            .current_function_params
                            .iter()
                            .find(|p| p.name == *name)
                            .is_some_and(|p| gen.is_type_copy(&p.type_));
                        if gen.inferred_mut_borrowed_params.contains(name) && !caller_param_is_copy
                        {
                            ownership = OwnershipMode::MutBorrowed;
                        }
                    }
                }
                if let Expression::Identifier { name, .. } = arg {
                    if gen.in_user_written_closure && gen.user_closure_params.contains(name) {
                        let pidx = sig.arg_param_index(i);
                        let callee_borrows = crate::ir::signature_bridge::call_site_expects_shared_borrow(
                            sig, pidx,
                        ) || sig.param_types.get(pidx).is_some_and(|t| {
                            matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        }) || sig
                            .emitted_rust_ref_params
                            .as_ref()
                            .and_then(|flags| flags.get(pidx).copied())
                            .unwrap_or(false)
                            || matches!(
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                    sig, i,
                                ),
                                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                            );
                        if callee_borrows {
                            if !arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                                arg_str = format!("&{arg_str}");
                            }
                            return vec![arg_str];
                        }
                    }
                }
                match ownership {
                        OwnershipMode::Borrowed if !has_ownership_collision => {
                            // PHASE 1: Generate &String parameters for correctness
                            // String literals need conversion: "foo" → &"foo".to_string()
                            let is_string_literal = matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            );

                            if is_string_literal {
                                let method_name = func_name.rsplit("::").next();
                                let receiver_type = func_name
                                    .split("::")
                                    .next()
                                    .filter(|q| q.chars().next().is_some_and(|c| c.is_ascii_uppercase()));
                                let needs_owned = crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion_with_enum(
                                    Some(sig),
                                    i,
                                    method_name,
                                    receiver_type,
                                    Some(&gen.enum_variant_types),
                                    func_name.split("::").next(),
                                );
                                if needs_owned {
                                    let base = if arg_str.ends_with(".to_string()") {
                                        arg_str[..arg_str.len() - 12].to_string()
                                    } else if arg_str.ends_with(".into()") {
                                        arg_str[..arg_str.len() - 7].to_string()
                                    } else {
                                        arg_str.clone()
                                    };
                                    return vec![format!(
                                        "{}.to_string()",
                                        base.trim_start_matches('&')
                                    )];
                                }

                                let param_is_str_ref = sig.param_type_for_arg(i).is_some_and(|t| {
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                                });

                                let asref_str_runtime = func_name
                                    .split("::")
                                    .next()
                                    .is_some_and(super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str);

                                if param_is_str_ref || asref_str_runtime {
                                    return vec![arg_str];
                                }

                                // &String param: string literal → &"lit".to_string()
                                let param_is_string_ref = sig.param_type_for_arg(i).is_some_and(|t| {
                                    crate::codegen::rust::string_utilities::param_is_rust_string_ref(t)
                                });
                                if param_is_string_ref {
                                    let base = arg_str.trim_start_matches('&');
                                    let base = if base.ends_with(".to_string()") {
                                        base.to_string()
                                    } else {
                                        format!("{}.to_string()", base)
                                    };
                                    return vec![format!("&{}", base)];
                                }

                                return vec![arg_str];
                            }

                            // TDD FIX: Check if parameter is already a reference type
                            // If param is &string / Phase-2 &str, don't add another & (would be &&str)
                            let is_param_already_ref =
                                if let Expression::Identifier { name, .. } = arg {
                                    gen.identifier_already_ref(name)
                                        || gen.current_function_params.iter().any(|param| {
                                            param.name == *name
                                                && matches!(
                                                    &param.type_,
                                                    Type::Reference(_)
                                                        | Type::MutableReference(_)
                                                )
                                        })
                                } else {
                                    false
                                };

                            // TDD FIX: Don't add & for Copy type parameters
                            // When signature says Borrowed but param type is Copy,
                            // codegen keeps it as owned (e.g., x: usize not x: &usize)
                            // So the call site should NOT add &
                            // BUT: Reference types (&Vec<T>, &[T]) are NOT treated as
                            // Copy here - if param type is &T, caller still needs &
                            let is_copy_param = sig
                                .param_type_for_arg(i)
                                .map(|t| {
                                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                        && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(t)
                                })
                                .unwrap_or(false);

                            // Temp variables hold owned `format!()` values. Skip auto-borrow
                            // only when the formal is owned String — `&str` formals need `&_temp`.
                            let is_temp_variable = arg_str.starts_with("_temp")
                                && arg_str.chars().skip(5).all(|c| c.is_numeric());
                            let temp_needs_shared_borrow = is_temp_variable
                                && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                    sig,
                                    sig.arg_param_index(i),
                                );

                            // Strip .clone() when destination wants Borrowed — pass &field, not &field.clone()
                            crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);

                            // Insert & if not already a reference and not a string literal and not a temp var
                            // THE WINDJAMMER WAY: Preserve user-written closure params
                            let _is_user_closure_param =
                                if let Expression::Identifier { name, .. } = arg {
                                    gen.in_user_written_closure && gen.user_closure_params.contains(name)
                                } else {
                                    false
                                };

                            let is_copy_scalar = is_copy_param
                                && sig.param_type_for_arg(i).is_some_and(|t| {
                                    matches!(
                                        t,
                                        Type::Custom(n)
                                            if matches!(
                                                n.as_str(),
                                                "i32" | "i64" | "u32" | "u64" | "usize"
                                                    | "f32" | "f64" | "bool" | "int" | "float"
                                                    | "byte"
                                            )
                                    )
                                });

                            if !expression_helpers::is_reference_expression(arg)
                                && !is_param_already_ref
                                && !is_copy_scalar
                                && !is_copy_literal
                                && (!is_temp_variable || temp_needs_shared_borrow)
                            {
                                crate::codegen::rust::rust_coercion_rules::Coercion::Borrow
                                    .apply(&mut arg_str);
                                return vec![arg_str];
                            } else if !expression_helpers::is_reference_expression(arg)
                                && !arg_str.starts_with('&')
                                && matches!(
                                    arg,
                                    Expression::MethodCall {
                                        method: m,
                                        ..
                                    } if m == "clone"
                                )
                            {
                                // `local.clone()` where callee expects `&T` → `&local`
                                crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                    &mut arg_str,
                                );
                                return vec![format!("&{}", arg_str)];
                            } else {
                                return vec![arg_str];
                            }
                        }
                        OwnershipMode::MutBorrowed if !has_ownership_collision => {
                            crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                                arg,
                                &mut arg_str,
                                &gen.current_function_params,
                                &gen.inferred_mut_borrowed_params,
                            );
                            return vec![arg_str];
                        }
                        OwnershipMode::Owned => {
                            // Static `Type::new` with owned `string` formals: pass `String` by
                            // value. Only add `&` when callee analysis converged to borrow (&str).
                            if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                                func_name,
                            ) && !sig.has_self_receiver && func_name.ends_with("::new")
                            {
                                if let Expression::Identifier { name, .. } = arg {
                                    let is_caller_owned_string = gen
                                        .current_function_params
                                        .iter()
                                        .any(|p| {
                                            p.name == *name
                                                && crate::codegen::rust::types::is_windjammer_text_type(
                                                    &p.type_,
                                                )
                                                && !matches!(
                                                    &p.type_,
                                                    Type::Reference(_)
                                                        | Type::MutableReference(_)
                                                )
                                        });
                                    if is_caller_owned_string && !arg_str.starts_with('&') {
                                        let effective =
                                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                                sig, i,
                                            );
                                        if matches!(
                                            effective,
                                            OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        ) {
                                            return vec![format!("&{}", arg_str)];
                                        }
                                    }
                                }
                            }

                            // Explicit `&x` / `&mut x` at call site → owned param needs deref (Copy) or clone.
                            if matches!(
                                arg,
                                Expression::Unary {
                                    op: UnaryOp::Ref | UnaryOp::MutRef,
                                    ..
                                }
                            ) && !arg_str.trim_start().starts_with('*')
                            {
                                let callee_wants_borrow = sig.param_types.get(i).is_some_and(|t| {
                                    matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                }) || matches!(
                                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                        sig, i,
                                    ),
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                                );
                                if callee_wants_borrow {
                                    return vec![arg_str];
                                }
                                // Signature-driven: callee formal type decides deref vs clone,
                                // not operand inference (explicit `&p` → owned `Light` needs clone).
                                let formal_idx = sig.arg_param_index(i);
                                let callee_formal_is_copy = sig
                                    .formal_param_type(formal_idx)
                                    .or_else(|| sig.param_types.get(formal_idx))
                                    .is_some_and(|t| match t {
                                        Type::Reference(inner) | Type::MutableReference(inner) => {
                                            gen.is_type_copy(inner)
                                        }
                                        other => gen.is_type_copy(other),
                                    });
                                if callee_formal_is_copy {
                                    // Explicit `&x` / `(&x)` → owned Copy formal: pass by value.
                                    // Expression gen may already have stripped `&`, leaving a
                                    // bare `x` — do NOT emit `*x` (E0614 on owned locals).
                                    if arg_str.starts_with("&mut ") {
                                        arg_str = arg_str["&mut ".len()..].to_string();
                                    } else if arg_str.starts_with('&') {
                                        arg_str = arg_str[1..].trim_start().to_string();
                                    } else if arg_str.starts_with('(') && arg_str.ends_with(')') {
                                        let inner = arg_str[1..arg_str.len() - 1].trim();
                                        if let Some(rest) = inner.strip_prefix("&mut ") {
                                            arg_str = rest.to_string();
                                        } else if let Some(rest) = inner.strip_prefix('&') {
                                            arg_str = rest.trim_start().to_string();
                                        }
                                    }
                                } else if !arg_str.ends_with(".clone()") {
                                    let inner = if arg_str.starts_with('(') && arg_str.ends_with(')') {
                                        let inner_expr = arg_str[1..arg_str.len() - 1].trim();
                                        if inner_expr.starts_with("&mut ") {
                                            inner_expr.strip_prefix("&mut ").unwrap_or(inner_expr)
                                        } else if inner_expr.starts_with('&') {
                                            inner_expr.strip_prefix('&').unwrap_or(inner_expr)
                                        } else {
                                            inner_expr
                                        }
                                    } else if arg_str.starts_with("&mut ") {
                                        arg_str.strip_prefix("&mut ").unwrap_or(&arg_str)
                                    } else if arg_str.starts_with('&') {
                                        arg_str.strip_prefix('&').unwrap_or(&arg_str)
                                    } else {
                                        arg_str.as_str()
                                    };
                                    arg_str = format!("{}.clone()", inner);
                                }
                                return vec![arg_str];
                            }

                            let param_is_str_ref = sig.param_types.get(i).is_some_and(|t| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                            })
                            && matches!(
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                    sig, i,
                                ),
                                OwnershipMode::Borrowed,
                            );
                            if param_is_str_ref {
                                // Owned String/local binding → borrow as &str via &String deref.
                                if !expression_helpers::is_reference_expression(arg)
                                    && !arg_str.starts_with('&')
                                {
                                    crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                        &mut arg_str,
                                    );
                                }
                                return vec![arg_str];
                            }

                            if let Expression::Identifier { name, .. } = arg {
                                if gen.in_user_written_closure
                                    && gen.user_closure_params.contains(name)
                                {
                                    let pidx = sig.arg_param_index(i);
                                    if (crate::ir::signature_bridge::call_site_expects_shared_borrow(
                                        sig, pidx,
                                    ) || sig.param_types.get(pidx).is_some_and(|t| {
                                        matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    }))
                                        && !arg_str.starts_with('&')
                                        && !arg_str.starts_with("&mut ")
                                    {
                                        arg_str = format!("&{arg_str}");
                                    }
                                    return vec![arg_str];
                                }
                                if !(gen.in_user_written_closure
                                    && gen.user_closure_params.contains(name))
                                {
                                    arg_str = gen.maybe_auto_clone(name, &arg_str);
                                }

                                // Find the parameter type
                                let param_type = gen
                                    .current_function_params
                                    .iter()
                                    .find(|p| &p.name == name)
                                    .map(|p| &p.type_);

                                // Check if it's a reference parameter (&str, &String, &T, &mut T)
                                let inner_from_ref = match param_type {
                                    Some(Type::Reference(inner)) => Some(inner.as_ref()),
                                    Some(Type::MutableReference(inner)) => Some(inner.as_ref()),
                                    _ => None,
                                };
                                if let Some(inner_type) = inner_from_ref {
                                    if matches!(inner_type, Type::String)
                                        && !arg_str.ends_with(".to_string()")
                                        && !arg_str.ends_with(".clone()")
                                    {
                                        arg_str = format!("{}.to_string()", arg_str);
                                    } else if gen.is_type_copy(inner_type)
                                        && !arg_str.trim_start().starts_with('*')
                                        && (arg_str.starts_with("&mut ")
                                            || (arg_str.starts_with('&')
                                                && !arg_str.starts_with("&&")))
                                    {
                                        arg_str = format!("*{}", arg_str);
                                    } else if !arg_str.ends_with(".clone()")
                                        && !arg_str.trim_start().starts_with('*')
                                    {
                                        arg_str = format!("{}.clone()", arg_str);
                                    }
                                } else {
                                    // TDD FIX: Check if it's from a borrowed iterator (for loop)
                                    // Example: for npc_id in npc_ids { Member::new(npc_id) }
                                    // npc_id is &String from iterator, needs .clone() for owned String
                                    //
                                    // CRITICAL: We're in OwnershipMode::Owned block, which means
                                    // the DESTINATION parameter wants an owned value (String, not &String).
                                    //
                                    // Windjammer `string` parameters lower to `&str`: `.clone()` keeps
                                    // `&str` (E0308). Use `.to_string()` for text types instead.
                                    let is_borrowed_iterator_var =
                                        gen.borrowed_iterator_vars.contains(name);

                                    let is_inferred_borrowed =
                                        gen.inferred_borrowed_params.contains(name);

                                    let is_inferred_mut_borrowed =
                                        gen.inferred_mut_borrowed_params.contains(name);

                                    // Borrowed formals are already `&T` in Rust — when converged
                                    // callee analysis expects borrow, do not `.clone()` for stale
                                    // Owned metadata (fps_camera::collides_aabb).
                                    if (is_inferred_borrowed || is_inferred_mut_borrowed)
                                        && !is_borrowed_iterator_var
                                        && !gen.str_ref_optimized_params.contains(name)
                                    {
                                        let lookup_name = if let Some(method) =
                                            func_name.strip_prefix("Self::")
                                        {
                                            if gen.in_impl_block {
                                                gen.current_struct_name.as_ref().map(|tn| {
                                                    format!("{tn}::{method}")
                                                })
                                            } else {
                                                None
                                            }
                                        } else {
                                            Some(func_name.to_string())
                                        };
                                        let receiver = lookup_name.as_ref().and_then(|n| {
                                            n.rsplit_once("::").map(|(qual, _)| qual)
                                        });
                                        let callee_expects_borrow = lookup_name
                                            .as_ref()
                                            .and_then(|n| {
                                                gen.resolve_call_signature_with_global(
                                                    n,
                                                    receiver,
                                                    arguments.len(),
                                                )
                                            })
                                            .is_some_and(|resolved| {
                                                matches!(
                                                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                                        &resolved.sig, i,
                                                    ),
                                                    OwnershipMode::Borrowed
                                                        | OwnershipMode::MutBorrowed
                                                )
                                            });
                                        if callee_expects_borrow {
                                            return vec![arg_str];
                                        }
                                    }

                                    if (is_borrowed_iterator_var
                                        || is_inferred_borrowed
                                        || is_inferred_mut_borrowed
                                        || gen.str_ref_optimized_params.contains(name))
                                        && !arg_str.ends_with(".clone()")
                                    {
                                        // `*ident` = owned Copy from &/&mut (see Identifier
                                        // in_owned_value_context); do not append .clone().
                                        if !arg_str.trim_start().starts_with('*') {
                                            let is_text = gen.str_ref_optimized_params.contains(name)
                                                || gen
                                                    .infer_expression_type(arg)
                                                    .as_ref()
                                                    .is_some_and(|t| {
                                                        crate::codegen::rust::types::is_windjammer_text_type(t)
                                                    });
                                            if is_text {
                                                let callee_wants_owned =
                                                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                                        sig, i,
                                                    ) == OwnershipMode::Owned;
                                                let arg_is_owned_string = gen
                                                    .current_function_params
                                                    .iter()
                                                    .find(|p| p.name == *name)
                                                    .is_some_and(|p| {
                                                        crate::codegen::rust::types::is_windjammer_text_type(
                                                            &p.type_,
                                                        ) && !matches!(
                                                            &p.type_,
                                                            Type::Reference(_)
                                                                | Type::MutableReference(_)
                                                        )
                                                    });
                                                if !callee_wants_owned && !arg_is_owned_string {
                                                    arg_str =
                                                        format!("{}.to_string()", arg_str);
                                                }
                                            } else if !is_text {
                                                let src_is_copy = gen
                                                    .infer_expression_type(arg)
                                                    .as_ref()
                                                    .map(|t| match t {
                                                        Type::Reference(inner)
                                                        | Type::MutableReference(inner) => {
                                                            gen.is_type_copy(inner)
                                                        }
                                                        other => gen.is_type_copy(other),
                                                    })
                                                    .unwrap_or(false);
                                                if src_is_copy {
                                                    if arg_str.starts_with("&mut ")
                                                        || (arg_str.starts_with('&')
                                                            && !arg_str.starts_with("&&"))
                                                    {
                                                        arg_str = format!("*{}", arg_str);
                                                    }
                                                } else {
                                                    // Borrowed from iterator or inferred - use .clone()
                                                    // This handles &T → T for non-text types
                                                    arg_str = format!("{}.clone()", arg_str);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            gen.maybe_clone_borrowed_field_for_owned_param(arg, &mut arg_str);
                            gen.maybe_clone_index_for_owned_param(arg, &mut arg_str);
                        }
                        _ => {
                            // Collision guard triggered: Borrowed or MutBorrowed
                            // with a signature collision. Don't apply auto-borrow;
                            // pass the argument as-is and let downstream Rust
                            // compilation determine the correct behavior.
                        }
                    }
            } else {
                // No signature found — still check auto-clone analysis.
                // The auto-clone analysis tracks data flow (value moved then
                // used later) independently of callee signatures (Some has no
                // registry entry).
                if let Expression::Identifier { name, .. } = arg {
                    arg_str = gen.maybe_auto_clone(name, &arg_str);
                }
            }

            // AUTO-CAST int → float: when parameter expects f32/f64 but argument is int.
            if let Some(ref sig) = signature {
                let method_part = func_name.rsplit("::").next().unwrap_or(func_name);
                let type_name = func_name
                    .contains("::")
                    .then(|| func_name.rsplit("::").nth(1).unwrap_or(""))
                    .filter(|tn| !tn.is_empty() && tn.chars().next().is_some_and(|c| c.is_ascii_uppercase()));
                let skip_cast = gen.should_skip_int_to_float_auto_cast_with_global(
                    type_name,
                    method_part,
                    Some(func_name),
                );
                if !skip_cast {
                    if let Some(param_ty) = sig.param_types.get(i) {
                        let arg_ty = gen.infer_expression_type(arg);
                        crate::codegen::rust::type_classification_utilities::maybe_cast_int_arg_to_float(
                            &mut arg_str, arg, param_ty, arg_ty.as_ref(),
                        );
                    }
                }
            }

            // Coerce owned String → &str when callee expects explicit &str (Phase 2 / FFI wrappers).
            // Also handle stale metadata with empty param_ownership: Windjammer `string`
            // params lower to borrowed &str at the callee definition site.
            // Skip when ownership collision detected — wrong module's metadata could apply bad &.
            if let Some(ref sig) = signature {
                if !has_ownership_collision {
                if let Some(param_ty) = sig.param_types.get(i) {
                    let param_is_str_ref = matches!(
                        param_ty,
                        Type::Reference(inner)
                            if matches!(**inner, Type::Custom(ref n) if n == "str")
                    );
                    let is_text_param = crate::codegen::rust::types::is_windjammer_text_type(param_ty);
                    let callee_borrows_string = !sig.is_extern
                        && !arg_str.contains("string_to_ffi(")
                        && (sig
                            .param_ownership
                            .get(i)
                            .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed))
                            || (sig.param_ownership.is_empty() && is_text_param));
                    let arg_already_ref =
                        if let Expression::Identifier { name, .. } = arg {
                            gen.identifier_already_ref(name)
                        } else {
                            false
                        };
                    let arg_is_owned_string = gen
                        .infer_expression_type(arg)
                        .is_some_and(|t| matches!(t, Type::String));
                    let arg_is_text_compatible = gen
                        .infer_expression_type(arg)
                        .as_ref()
                        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
                    if (param_is_str_ref || callee_borrows_string)
                        && arg_is_text_compatible
                        && !arg_is_owned_string
                        && !arg_str.contains("string_to_ffi(")
                        && !arg_str.starts_with('&')
                        && !arg_already_ref
                        && !matches!(
                            arg,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        )
                    {
                        arg_str = format!("&{}", arg_str);
                    }
                }
                }
            }

            // Runtime std modules (db, env, …): Rust takes &str while WJ declares owned string.
            if !arg_str.starts_with('&') {
                let module = func_name.split("::").next().unwrap_or("");
                if super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str(
                    module,
                ) {
                    let param_is_string = signature
                        .as_ref()
                        .and_then(|sig| sig.param_types.get(i))
                        .is_some_and(
                            crate::codegen::rust::string_utilities::param_is_owned_string_type,
                        );
                    if param_is_string
                        && matches!(
                            arg,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        )
                    {
                        arg_str = format!("&{}", arg_str);
                    }
                }
            }

            // Runtime std module auto-borrow: windjammer_runtime functions take &T
            // for non-Copy struct params (e.g. json::get(&value, ...) not json::get(value, ...)).
            // WJ stdlib declares owned params; the Rust side uses references.
            if !arg_str.starts_with('&') {
                if crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    &gen.signature_registry,
                    func_name,
                    signature.as_ref(),
                    i,
                ) {
                    let already_ref = if let Expression::Identifier { name, .. } = arg {
                        gen.binding_emits_as_rust_shared_ref(name)
                    } else {
                        false
                    };
                    if !already_ref {
                        arg_str = format!("&{}", arg_str);
                    }
                }
            }

            if let Some(ref sig) = signature {
                if !is_extern_call && !sig.is_extern && !has_ownership_collision {
                    let param_idx = sig.arg_param_index(i);
                    let callee_param_is_copy = sig
                        .formal_param_type(param_idx)
                        .is_some_and(|t| gen.is_type_copy(t));
                    let callee_param_type_is_ref = sig
                        .param_types
                        .get(param_idx)
                        .is_some_and(|t| {
                            matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        });
                    let needs_mut_borrow = matches!(
                        crate::codegen::rust::call_site_borrow::effective_ownership_for_call_arg(
                            sig, i
                        ),
                        OwnershipMode::MutBorrowed
                    );
                    if !callee_param_is_copy || needs_mut_borrow || callee_param_type_is_ref {
                        let method_name = func_name.rsplit("::").next().unwrap_or(func_name);
                        let arg_already_rust_ref = matches!(
                            arg,
                            Expression::Identifier { name, .. }
                                if gen.identifier_already_ref(name)
                                    || gen.str_ref_optimized_params.contains(name.as_str())
                        );
                        let formal_is_copy = callee_param_is_copy;
                        let decision =
                            crate::codegen::rust::call_site_borrow::should_borrow_at_call_site_with_copy_check(
                                sig, i, arg, &arg_str, method_name, arg_already_rust_ref, None, formal_is_copy,
                            );
                        crate::codegen::rust::call_site_borrow::apply_call_site_borrow(
                            &decision, &mut arg_str,
                        );
                    }
                }
            }

            // Final guard: non-reference Copy-type formals should never have spurious `&` added.
            // Exception: `&mut` for mut-borrowed Copy params (e.g. increment(&mut counter)).
            if let Some(ref sig) = signature {
                let pidx = sig.arg_param_index(i);
                let needs_mut_borrow = matches!(
                    crate::codegen::rust::call_site_borrow::effective_ownership_for_call_arg(
                        sig, i
                    ),
                    OwnershipMode::MutBorrowed
                );
                let formal_is_non_ref_copy = sig
                    .formal_param_type(pidx)
                    .is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && gen.is_type_copy(t)
                    });
                let is_coll_key = crate::codegen::rust::stdlib_method_traits::is_map_key_method(
                    func_name.rsplit("::").next().unwrap_or(func_name),
                ) && i == 0;
                let formal_is_owned_string = sig.formal_param_type(pidx).is_some_and(|t| {
                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        && crate::codegen::rust::types::is_windjammer_text_type(t)
                });
                if (formal_is_non_ref_copy || formal_is_owned_string)
                    && !is_coll_key && !needs_mut_borrow
                    && arg_str.starts_with('&') && !arg_str.starts_with("&mut ")
                {
                    arg_str = arg_str[1..].to_string();
                }
            }

            if crate::codegen::rust::typed_lowering::is_typed_lowering_enabled() {
                if let Some(ref sig) = signature {
                    let pidx = sig.arg_param_index(i);
                    let is_formal_copy = sig
                        .formal_param_type(pidx)
                        .is_some_and(|t| {
                            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                && gen.is_type_copy(t)
                        });
                    let is_coll_key = crate::codegen::rust::stdlib_method_traits::is_map_key_method(
                        func_name.rsplit("::").next().unwrap_or(func_name),
                    ) && i == 0;
                    crate::codegen::rust::typed_lowering::correct_legacy_output(
                        sig,
                        i,
                        arg,
                        &mut arg_str,
                        is_formal_copy,
                        is_coll_key,
                    );
                }
            }

            let simple_fn = func_name.rsplit("::").next().unwrap_or(func_name);
            let owned_formal = [
                gen.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(func_name)),
                gen.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(simple_fn)),
                gen.signature_registry.get_signature(func_name),
                gen.signature_registry.get_signature(simple_fn),
                signature.as_ref(),
            ]
            .iter()
            .flatten()
            .any(|sig| {
                let pidx = sig.arg_param_index(i);
                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
                    || crate::ir::signature_bridge::call_site_expects_owned_pass(sig, pidx)
            });
            let sig_for_final_mut = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                gen.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(func_name).cloned()),
                gen.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(simple_fn).cloned()),
                signature.as_ref().cloned(),
                gen.signature_registry.get_signature(func_name).cloned(),
            ]);
            let callee_wants_mut_arg = !owned_formal
                && (gen
                    .function_emitted_mut_arg_indices
                    .get(func_name)
                    .or_else(|| {
                        gen.function_emitted_mut_arg_indices.get(simple_fn)
                    })
                    .is_some_and(|indices| indices.contains(&i))
                    || sig_for_final_mut.as_ref().is_some_and(|sig| {
                        crate::codegen::rust::call_signature_resolution::callee_user_arg_expects_mut_borrow(
                            sig, i,
                        )
                    }));
            if owned_formal
                && (arg_str.starts_with("&mut ")
                    || (arg_str.starts_with('&') && !arg_str.starts_with("&mut ")))
            {
                arg_str = crate::codegen::rust::expression_utilities::borrow_base_expr(&arg_str)
                    .to_string();
            } else if (!has_ownership_collision || callee_wants_mut_arg)
                && callee_wants_mut_arg
                && !arg_str.starts_with("&mut ")
                && crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(arg)
            {
                crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                    arg,
                    &mut arg_str,
                    &gen.current_function_params,
                    &gen.inferred_mut_borrowed_params,
                );
            } else if !has_ownership_collision {
                if let Some(ref sig) = sig_for_final_mut {
                let pidx = sig.arg_param_index(i);
                let needs_mut_borrow = !owned_formal
                    && (matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, i,
                    ),
                    OwnershipMode::MutBorrowed,
                ) || (sig.param_types.get(pidx).is_some_and(|t| {
                    matches!(t, Type::MutableReference(_))
                }) && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, pidx,
                )) || (matches!(
                    sig.param_ownership.get(pidx),
                    Some(OwnershipMode::MutBorrowed)
                ) && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, pidx,
                )));
                if needs_mut_borrow
                    && !arg_str.starts_with("&mut ")
                    && crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(arg)
                {
                    crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                        arg,
                        &mut arg_str,
                        &gen.current_function_params,
                        &gen.inferred_mut_borrowed_params,
                    );
                }
                // Copy aggregates emit owned formals — strip stale `&mut` at call sites.
                if arg_str.starts_with("&mut ") {
                    if let Some(formal) = sig.formal_param_type(pidx) {
                        let bare = match formal {
                            Type::Reference(inner) | Type::MutableReference(inner) => {
                                inner.as_ref()
                            }
                            other => other,
                        };
                        if crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
                            && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                            && !matches!(
                                formal,
                                Type::Reference(_) | Type::MutableReference(_)
                            )
                        {
                            arg_str = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                &arg_str,
                            )
                            .to_string();
                        }
                    }
                }
                }
            }

            if !arg_str.starts_with('&')
                && matches!(
                    arg,
                    Expression::Identifier { .. } | Expression::FieldAccess { .. }
                )
                && !matches!(
                    arg,
                    Expression::Identifier { name, .. } if gen.identifier_already_ref(name)
                )
            {
                if signature.as_ref().is_some_and(|sig| {
                    crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow(
                        Some(sig),
                        i,
                    )
                }) {
                    arg_str = format!("&{arg_str}");
                }
            }

            if matches!(arg, Expression::FieldAccess { .. } | Expression::Index { .. })
                && !arg_str.starts_with('&')
                && !arg_str.ends_with(".clone()")
            {
                let borrow_sig = if func_name.starts_with("Self::") {
                    gen.current_struct_name.as_ref().and_then(|tn| {
                        func_name.strip_prefix("Self::").and_then(|method| {
                            gen.lookup_method_signature(tn, method)
                                .map(|ms| ms.to_function_signature())
                        })
                    })
                } else {
                    None
                }
                .or_else(|| signature.clone());
                if let Some(sig) = borrow_sig.as_ref() {
                    let idx = sig.arg_param_index(i);
                    let ownership =
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                            sig, i,
                        );
                    let param_is_str_ref = sig.param_types.get(idx).is_some_and(|t| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    });
                    let emits_shared_ref =
                        crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            sig, idx,
                        );
                    let wants_owned = matches!(ownership, OwnershipMode::Owned)
                        && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig, idx,
                        )
                        && !param_is_str_ref
                        && !emits_shared_ref;
                    let wants_mut = matches!(ownership, OwnershipMode::MutBorrowed)
                        || sig.param_types.get(idx).is_some_and(|t| {
                            matches!(t, Type::MutableReference(_))
                        });
                    let wants_shared_borrow = !wants_owned
                        && !wants_mut
                        && (matches!(ownership, OwnershipMode::Borrowed)
                            || param_is_str_ref
                            || emits_shared_ref
                            || matches!(
                                sig.param_types.get(idx),
                                Some(Type::Reference(_))
                            ))
                        && (sig.formal_param_type(idx).is_some_and(
                            crate::codegen::rust::types::is_windjammer_text_type,
                        ) || param_is_str_ref
                            || sig.param_types.get(idx).is_some_and(|t| {
                                crate::codegen::rust::types::is_windjammer_text_type(t)
                                    || matches!(t, Type::Reference(_))
                            }));
                    if wants_mut && !arg_str.starts_with("&mut ") {
                        crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                            arg,
                            &mut arg_str,
                            &gen.current_function_params,
                            &gen.inferred_mut_borrowed_params,
                        );
                    } else if wants_shared_borrow {
                        if arg_str.ends_with(".clone()") {
                            arg_str = arg_str
                                .trim_end_matches(".clone()")
                                .trim()
                                .to_string();
                        }
                        if !arg_str.starts_with('&') {
                            arg_str = format!("&{arg_str}");
                        }
                    }
                }
            }

            if has_ownership_collision {
                let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                let collision_sig = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(func_name).cloned()),
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned()),
                    gen.signature_registry.get_signature(func_name).cloned(),
                    gen.signature_registry.get_signature(simple).cloned(),
                    signature.clone(),
                ]);
                let collision_pidx = collision_sig
                    .as_ref()
                    .map(|s| s.arg_param_index(i))
                    .unwrap_or(i);
                let emits_shared_ref = collision_sig.as_ref().is_some_and(|s| {
                    crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        s,
                        collision_pidx,
                    )
                });
                let preserve_text_borrow = emits_shared_ref
                    && collision_sig.as_ref().is_some_and(|s| {
                        crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                            s, collision_pidx,
                        ) || s.param_types.get(collision_pidx).is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                                || matches!(
                                    t,
                                    Type::Reference(inner)
                                        if crate::codegen::rust::types::is_windjammer_text_type(inner)
                                )
                        })
                    });
                // Homonym collisions (`check`, `process`, …) must not strip `&` when
                // codegen confirmed a shared-ref formal (`emitted_rust_ref_params` /
                // converged `Reference(Item)`). Previously only text/`&str` was
                // preserved — `check(item: &Item)` lost its call-site `&` (bug_e0308).
                if emits_shared_ref {
                    if preserve_text_borrow {
                        if let Some(stripped) = arg_str.strip_suffix(".to_string()") {
                            arg_str = stripped.to_string();
                        }
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut arg_str,
                        );
                    }
                    if let Some(ref sig) = collision_sig {
                        if !arg_str.starts_with('&')
                            && !expression_helpers::is_reference_expression(arg)
                        {
                            let arg_already_rust_ref = matches!(
                                arg,
                                Expression::Identifier { name, .. }
                                    if gen.identifier_already_ref(name)
                                        || gen.str_ref_optimized_params.contains(name.as_str())
                                        || gen.inferred_borrowed_params.contains(name)
                            );
                            let formal_is_copy = sig.param_type_for_arg(i).is_some_and(|t| {
                                gen.is_type_copy(t)
                            });
                            let decision = crate::codegen::rust::call_site_borrow::should_borrow_at_call_site_with_copy_check(
                                sig,
                                i,
                                arg,
                                &arg_str,
                                simple,
                                arg_already_rust_ref,
                                None,
                                formal_is_copy,
                            );
                            crate::codegen::rust::call_site_borrow::apply_call_site_borrow(
                                &decision,
                                &mut arg_str,
                            );
                        }
                    }
                } else {
                    crate::codegen::rust::call_signature_resolution::strip_collision_blocked_call_site_coercions(
                        &mut arg_str,
                    );
                }
            } else {
                // Even without an ownership collision, strip literal `.to_string()` when the
                // defining module refreshed a plain `string` formal to `&str` (WDB-048).
                let simple = func_name.rsplit("::").next().unwrap_or(func_name);
                let mut text_sig = crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(func_name).cloned()),
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned()),
                    gen.signature_registry.get_signature(func_name).cloned(),
                    gen.signature_registry.get_signature(simple).cloned(),
                    signature.clone(),
                ]);
                let pidx = text_sig
                    .as_ref()
                    .map(|s| s.arg_param_index(i))
                    .unwrap_or(i);
                for challenger in [
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(func_name)),
                    gen.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple)),
                    gen.signature_registry.get_signature(func_name),
                    gen.signature_registry.get_signature(simple),
                ] {
                    text_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                        text_sig, challenger, pidx,
                    );
                }
                if text_sig.as_ref().is_some_and(|s| {
                    crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        s, s.arg_param_index(i),
                    )
                }) {
                    if let Some(stripped) = arg_str.strip_suffix(".to_string()") {
                        arg_str = stripped.to_string();
                    }
                }
            }

            vec![arg_str]
        })
        .collect()
}
