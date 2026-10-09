//! `Call(FieldAccess)` lowering: treat as method call with signature-aware arguments.

use crate::analyzer::OwnershipMode;
use crate::codegen::rust::call_signature_resolution;
use crate::parser::*;

use super::super::CodeGenerator;
use super::argument_generation;

/// Parser sometimes emits `Call { function: FieldAccess { .. }, args }` instead of `MethodCall`.
#[allow(clippy::too_many_arguments)]
pub(in crate::codegen::rust) fn generate_call_on_field_access<'ast>(
    gen: &mut CodeGenerator<'ast>,
    call_obj: &'ast Expression<'ast>,
    call_method: &str,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> String {
    let type_name = gen
        .mc_infer_method_receiver_type_name(call_obj)
        .or_else(|| gen.infer_type_name(call_obj))
        .or_else(|| {
            gen.infer_expression_type(call_obj)
                .and_then(|t| CodeGenerator::type_to_name(&t))
        });
    let registry = gen
        .global_signature_registry()
        .unwrap_or(&gen.signature_registry);
    let is_type_preserving =
        crate::codegen::rust::stdlib_method_traits::method_is_type_preserving_qualified(
            call_method,
            type_name.as_deref(),
            registry,
        );

    // Same receiver lowering as `MethodCall` — parser often emits
    // `Call(FieldAccess)` for `row.get_string(col)`. Without method-receiver
    // context, auto-clone inserts `row.clone().get_string` even for `&self`.
    let mut obj_str = gen.mc_build_method_receiver_string(call_obj, call_method);
    if is_type_preserving {
        crate::codegen::rust::string_utilities::strip_redundant_auto_clone_before_explicit_clone(
            &mut obj_str,
            call_method,
        );
    }

    // Prefer converged signature_registry (Owned consumers like MannequinMesh::generate)
    // over per-body method_signatures_by_type (may infer Borrowed when a formal is reused
    // inside the callee). Method registry remains fallback for stdlib / not-yet-registered.
    let from_registry = type_name.as_ref().and_then(|tn| {
        call_signature_resolution::resolve_method_for_call_site_in_module(
            &gen.signature_registry,
            gen.global_signature_registry(),
            tn,
            call_method,
            arguments.len(),
            gen.current_caller_module_path().as_deref(),
        )
    });

    let from_method_registry = type_name.as_ref().and_then(|tn| {
        gen.lookup_method_signature(tn, call_method).and_then(|ms| {
            let sig = ms.to_function_signature();
            if call_signature_resolution::validate_arg_count(&sig, arguments.len()) {
                Some(call_signature_resolution::ResolvedSignature {
                    sig,
                    qualified_key: format!("{tn}::{call_method}"),
                    resolution_method: call_signature_resolution::ResolutionMethod::MethodRegistry,
                    has_collision: false,
                })
            } else {
                None
            }
        })
    });

    let resolved = call_signature_resolution::pick_best_resolved_signature(
        from_registry,
        from_method_registry,
    );
    let method_signature = resolved.as_ref().map(|r| {
        let mut sig = r.sig.clone();
        call_signature_resolution::apply_trait_owned_string_call_site_contracts(
            &gen.signature_registry,
            call_method,
            &mut sig,
        );
        if let Some(global) = gen.global_signature_registry() {
            call_signature_resolution::apply_trait_owned_string_call_site_contracts(
                global,
                call_method,
                &mut sig,
            );
        }
        call_signature_resolution::finalize_call_site_signature(sig)
    });

    let runtime_module = match call_obj {
        Expression::Identifier { name, .. } => {
            // Only imported `use std::…` modules — never bare-name list matches
            // (`json` / `server` locals must not become `json::` / `server::`).
            if gen.is_imported_runtime_std_module(name) {
                Some(name.as_str())
            } else {
                None
            }
        }
        _ => None,
    };

    // `json.parse(...)` / `fs.write(...)` are Call(FieldAccess) with a module receiver —
    // type inference often yields no receiver type, so resolve `module::method` directly.
    let method_signature = method_signature.or_else(|| {
        let module = runtime_module?;
        let key = format!("{module}::{call_method}");
        // Multipass analyzes `std/*.wj` stubs into the crate registry without a layered
        // runtime fallback — `refresh_call_site_signature_for_arg` challenges
        // `SignatureRegistry::stdlib()` so `&str`/`AsRef<str>` beat owned WJ formals
        // (`strings::split` delimiter, `fs::write` path, …).
        gen.refresh_call_site_signature_for_arg(
            gen.get_signature_with_global(&key).cloned(),
            &key,
            0,
        )
        .map(call_signature_resolution::finalize_call_site_signature)
    });

    // Match bindings (`Ok(mut app)` after `Mutex::lock`) often have no inferred
    // receiver type, so `Type::method` resolution above is skipped. Still pick a
    // codegen-refreshed `*::method` (hexagonal `app.handle` → `App::handle` `&str`).
    // Skip bare `::get` scramble when the receiver is a map Deref wrapper — that
    // path often picks an Owned homonym (`key.to_string()`); HashMap bridge below.
    let method_signature = method_signature.or_else(|| {
        let base = type_name
            .as_deref()
            .map(|tn| tn.split('<').next().unwrap_or(tn));
        if base
            .is_some_and(crate::codegen::rust::stdlib_method_traits::is_map_deref_wrapper_type_name)
        {
            return None;
        }
        let suffix = format!("::{call_method}");
        let mut candidates: Vec<Option<crate::analyzer::FunctionSignature>> = Vec::new();
        let push_matching =
            |reg: &crate::analyzer::SignatureRegistry,
             out: &mut Vec<Option<crate::analyzer::FunctionSignature>>| {
                for (key, sig) in &reg.signatures {
                    if key.ends_with(&suffix)
                        && call_signature_resolution::validate_arg_count(sig, arguments.len())
                    {
                        out.push(Some(sig.clone()));
                    }
                }
            };
        if let Some(g) = gen.global_signature_registry() {
            push_matching(g, &mut candidates);
        }
        push_matching(&gen.signature_registry, &mut candidates);
        crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature(candidates)
            .map(call_signature_resolution::finalize_call_site_signature)
    });

    // Always overlay defining-module `emitted_rust_ref_params` before IR arg coercion.
    let method_signature = method_signature.map(|mut sig| {
        let mut keys: Vec<String> = Vec::new();
        if let Some(tn) = type_name.as_deref() {
            keys.push(format!("{tn}::{call_method}"));
        }
        if !sig.name.is_empty() {
            keys.push(sig.name.clone());
        }
        keys.push(format!("::{call_method}"));
        crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
            &mut sig,
            &gen.signature_registry,
            &keys,
        );
        if let Some(global) = gen.global_signature_registry() {
            crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                &mut sig, global, &keys,
            );
            // Suffix keys alone may not hit `get_signature` — pull shared-ref winners.
            let suffix = format!("::{call_method}");
            for (key, gsig) in &global.signatures {
                if key.ends_with(&suffix)
                    && call_signature_resolution::validate_arg_count(gsig, arguments.len())
                    && crate::codegen::rust::signature_promotion::shared_ref_emission_beats(
                        gsig, &sig,
                    )
                {
                    crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                        &mut sig, gsig,
                    );
                }
            }
        }
        // Trait AST `string` must win after shared-ref refresh from impl keys
        // (`SeedCredentialAuthenticator::authenticate` body-converged `&str`).
        call_signature_resolution::apply_trait_owned_string_call_site_contracts(
            &gen.signature_registry,
            call_method,
            &mut sig,
        );
        if let Some(global) = gen.global_signature_registry() {
            call_signature_resolution::apply_trait_owned_string_call_site_contracts(
                global,
                call_method,
                &mut sig,
            );
        }
        call_signature_resolution::finalize_call_site_signature(sig)
    });

    // P3.576 / wj-sync: after refresh overlay, replace poisoned Owned wrapper/`*::get`
    // with stdlib `HashMap::get` (Borrowed `&K`) for Call(FieldAccess) match scrutinees.
    // Do not treat owned Copy formals (`usize` / `i64` on Vec::get / Store::get) as
    // poisoned map keys (P3.649–651).
    let method_signature = {
        let hashmap_bridge =
            crate::codegen::rust::stdlib_method_traits::hashmap_key_method_signature_for_wrapper(
                call_method,
                type_name.as_deref(),
                &gen.signature_registry,
            )
            .map(call_signature_resolution::finalize_call_site_signature);
        match (method_signature, hashmap_bridge) {
            (Some(cur), Some(hashmap)) => {
                let pidx = cur.arg_param_index(0);
                let owned_copy = cur
                    .formal_param_type(pidx)
                    .or_else(|| cur.param_types.get(pidx))
                    .is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && crate::type_classification::is_copy_pass_by_value_formal(t)
                    });
                let receiver_base = type_name
                    .as_deref()
                    .map(|rt| rt.split('<').next().unwrap_or(rt));
                let receiver_is_non_map = receiver_base.is_some_and(|base| {
                    !crate::type_classification::is_map_type_name(base)
                        && !crate::type_classification::is_set_type_name(base)
                        && !crate::codegen::rust::stdlib_method_traits::is_map_deref_wrapper_type_name(
                            base,
                        )
                });
                let poisoned_owned =
                    matches!(cur.param_ownership.get(pidx), Some(OwnershipMode::Owned))
                        && !matches!(
                            cur.param_types.get(pidx),
                            Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                        )
                        && !owned_copy
                        && !receiver_is_non_map;
                // `Ok(g) => g.get(key)` often has `rt=None` and suffix-resolves to
                // `Vec::get` (owned Copy index). A known non-map receiver keeps that
                // formal. An unknown or guard receiver with an owned-Copy homonym
                // must use the HashMap `&K` bridge. `hashmap_bridge` is already
                // limited to map-key methods, so the leaf name is not a second filter.
                let vec_index_homonym = owned_copy
                    && !receiver_is_non_map
                    && (receiver_base.is_none() || receiver_base.is_some_and(|base| {
                        crate::codegen::rust::stdlib_method_traits::is_map_deref_wrapper_type_name(
                            base,
                        )
                    }) || cur.name.contains("Vec::")
                        || cur.name.contains("slice::"));
                if poisoned_owned || vec_index_homonym {
                    Some(hashmap)
                } else {
                    Some(cur)
                }
            }
            (None, Some(hashmap)) => {
                let receiver_base = type_name
                    .as_deref()
                    .map(|rt| rt.split('<').next().unwrap_or(rt));
                let receiver_is_non_map = receiver_base.is_some_and(|base| {
                    !crate::type_classification::is_map_type_name(base)
                        && !crate::type_classification::is_set_type_name(base)
                        && !crate::codegen::rust::stdlib_method_traits::is_map_deref_wrapper_type_name(
                            base,
                        )
                });
                if receiver_is_non_map {
                    None
                } else {
                    Some(hashmap)
                }
            }
            (other, _) => other,
        }
    };

    let mut args: Vec<String> = {
        let prev_float = gen.push_float_method_argument_context(call_method, call_obj);
        let built = if let Some(ref sig) = method_signature {
            argument_generation::field_access_method_args_with_signature(
                gen,
                sig,
                call_method,
                &method_signature,
                &type_name,
                call_obj,
                runtime_module,
                arguments,
            )
        } else {
            argument_generation::field_access_method_args_fallback(
                gen,
                call_method,
                &type_name,
                call_obj,
                runtime_module,
                arguments,
            )
        };
        gen.assignment_float_target_type = prev_float;
        built
    };

    // P3.635 / WDB-134: match scrutinees use Call(FieldAccess); belt map-key borrow
    // only for map/set/wrapper receivers — never Vec::get / Store::get (P3.649–651).
    if crate::codegen::rust::stdlib_method_traits::is_map_key_method(call_method) {
        let receiver_base = type_name
            .as_deref()
            .map(|rt| rt.split('<').next().unwrap_or(rt));
        let receiver_is_mapish = receiver_base.is_some_and(|base| {
            crate::type_classification::is_map_type_name(base)
                || crate::type_classification::is_set_type_name(base)
                || crate::codegen::rust::stdlib_method_traits::is_map_deref_wrapper_type_name(base)
        });
        if receiver_is_mapish {
            let qualified_for_key =
                crate::codegen::rust::stdlib_method_traits::module_qualified_method_name(
                    type_name.as_deref(),
                    call_obj,
                    call_method,
                    |name| gen.is_imported_runtime_std_module(name),
                );
            let sig_for_key = method_signature.clone().or_else(|| {
                crate::codegen::rust::stdlib_method_traits::hashmap_key_method_signature_for_wrapper(
                    call_method,
                    type_name.as_deref(),
                    &gen.signature_registry,
                )
            });
            if let Some(ref sig_for_key) = sig_for_key {
                for (i, arg_str) in args.iter_mut().enumerate() {
                    let Some((_, arg_expr)) = arguments.get(i) else {
                        continue;
                    };
                    gen.finalize_post_ir_collection_key_arg(
                        arg_str,
                        arg_expr,
                        i,
                        call_method,
                        &qualified_for_key,
                        type_name.as_deref(),
                        sig_for_key,
                    );
                }
            }
        } else if let Some(sig) = method_signature.as_ref() {
            // Non-map receiver: a leading `&` on an owned Copy formal is a stale
            // map-key borrow. The formal decides; leaf names do not.
            for (i, arg_str) in args.iter_mut().enumerate() {
                if !arg_str.starts_with('&') || arg_str.starts_with("&mut ") {
                    continue;
                }
                if gen.is_collection_key_lookup_at_site(sig, i, type_name.as_deref()) {
                    continue;
                }
                if crate::codegen::rust::call_site_borrow::callee_user_arg_bare_formal_is_copy_pass_by_value(
                    sig, i,
                ) {
                    *arg_str = crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str)
                        .to_string();
                }
            }
        }
    }

    // Runtime std modules where WJ declares owned aggregates but Rust takes references.
    if let Expression::Identifier { name: obj_name, .. } = call_obj {
        let registry = gen
            .global_signature_registry()
            .unwrap_or(&gen.signature_registry);
        let callee_key = format!("{obj_name}::{call_method}");
        for (i, arg_str) in args.iter_mut().enumerate() {
            if arg_str.starts_with('&') {
                continue;
            }
            let Some((_, arg_expr)) = arguments.get(i) else {
                continue;
            };
            if !matches!(
                arg_expr,
                Expression::Identifier { .. } | Expression::FieldAccess { .. }
            ) {
                continue;
            }
            if matches!(
                arg_expr,
                Expression::Identifier { name, .. } if gen.identifier_already_ref(name)
            ) {
                continue;
            }
            if crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                registry,
                &callee_key,
                method_signature.as_ref(),
                i,
            ) {
                *arg_str = format!("&{arg_str}");
            }
        }
    }

    // Trait AST `string` must be last writer before the borrow post-pass —
    // impl shared-ref refresh / `shared_ref_emission_beats` can re-pollute.
    let method_signature = method_signature.map(|mut sig| {
        call_signature_resolution::apply_trait_owned_string_call_site_contracts(
            &gen.signature_registry,
            call_method,
            &mut sig,
        );
        if let Some(global) = gen.global_signature_registry() {
            call_signature_resolution::apply_trait_owned_string_call_site_contracts(
                global,
                call_method,
                &mut sig,
            );
        }
        sig
    });

    // Borrow owned String args when the resolved signature says the callee
    // takes `string` by borrow (lowers to `&str` in Rust).
    // Skip when ownership collision detected for this method name.
    let has_method_collision = gen.has_collision_with_global(call_method);
    if let Some(ref sig) = method_signature {
        if has_method_collision {
            // Collision detected — skip post-processing auto-borrow entirely.
        } else {
            let callee_is_extern = sig.is_extern;
            args = args
            .iter()
            .enumerate()
            .map(|(i, arg_str)| {
                let sig_param_idx = sig.arg_param_index(i);
                if call_signature_resolution::global_trait_owned_plain_string_arg(
                    &gen.signature_registry,
                    call_method,
                    i,
                ) || gen.global_signature_registry().is_some_and(|g| {
                    call_signature_resolution::global_trait_owned_plain_string_arg(
                        g, call_method, i,
                    )
                }) {
                    return arg_str.clone();
                }
                // Plain owned `string` trait formals pass `String` at the call site.
                if sig.formal_param_type(sig_param_idx).is_some_and(|t| {
                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        && crate::codegen::rust::types::is_windjammer_text_type(t)
                }) {
                    let caller_passes_str_slice = arguments.get(i).is_some_and(|(_, arg_expr)| {
                        if let Expression::Identifier { name, .. } = *arg_expr {
                            gen.str_ref_optimized_params.contains(name.as_str())
                                || gen.inferred_borrowed_params.contains(name)
                                || gen.current_function_params.iter().any(|p| {
                                    p.name == *name
                                        && matches!(
                                            &p.type_,
                                            Type::Reference(inner)
                                                if matches!(
                                                    **inner,
                                                    Type::Custom(ref s) if s == "str"
                                                )
                                        )
                                })
                        } else {
                            false
                        }
                    });
                    if caller_passes_str_slice && !arg_str.ends_with(".to_string()") {
                        return format!("{}.to_string()", arg_str);
                    }
                    return arg_str.clone();
                }
                let borrow = !callee_is_extern
                    && !arg_str.contains("string_to_ffi(")
                    && matches!(
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                            sig, i,
                        ),
                        OwnershipMode::Borrowed,
                    )
                    && sig
                        .param_types
                        .get(sig_param_idx)
                        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
                let arg_is_copy_scalar = arguments.get(i).is_some_and(|(_, arg_expr)| {
                    if let Some(t) = gen.infer_expression_type(arg_expr) {
                        gen.is_type_copy(&t)
                            && !crate::codegen::rust::types::is_windjammer_text_type(&t)
                    } else if let Expression::Identifier { name, .. } = *arg_expr {
                        gen.current_function_params
                            .iter()
                            .find(|p| p.name == *name)
                            .is_some_and(|p| {
                                gen.is_type_copy(&p.type_)
                                    && !crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                            })
                    } else {
                        false
                    }
                });
                let arg_is_text_compatible = arguments.get(i).is_some_and(|(_, arg_expr)| {
                    gen.infer_expression_type(arg_expr)
                        .as_ref()
                        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                });
                if borrow
                    && arg_is_text_compatible
                    && !arg_is_copy_scalar
                    && !arg_str.starts_with('&')
                    && !arg_str.starts_with('"')
                {
                    let arg_is_str_param = arguments.get(i).is_some_and(|(_, arg_expr)| {
                        if let Expression::Identifier { name, .. } = *arg_expr {
                            gen.identifier_already_ref(name)
                        } else if let Expression::Unary {
                            op: UnaryOp::Ref,
                            operand,
                            ..
                        } = *arg_expr
                        {
                            if let Expression::Identifier { name, .. } = &**operand {
                                gen.identifier_already_ref(name)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    });
                    if arg_is_str_param {
                        arg_str.clone()
                    } else {
                        let mut borrowed = arg_str.clone();
                        crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                            &mut borrowed,
                        );
                        borrowed
                    }
                } else {
                    arg_str.clone()
                }
            })
            .collect();
        }
    }

    // Type constructors: Vec::new(), HashMap::with_capacity() — not instance methods.
    // Runtime std modules (`http.get`, `process.exit`) use path `::` when the
    // identifier is an imported/scanned module (signature `{module}::{method}`).
    let separator = match call_obj {
        Expression::Identifier { name, .. } => {
            if CodeGenerator::is_enum_variant_qualified_path(name)
                || name.chars().next().is_some_and(|c| c.is_uppercase())
                || runtime_module.is_some()
            {
                "::"
            } else {
                "."
            }
        }
        _ => ".",
    };

    for (i, arg_str) in args.iter_mut().enumerate() {
        if let Some((_, arg_expr)) = arguments.get(i) {
            // P3.658: only *emitted* shared-ref formals (`key: &str`) skip the map-key
            // `&` belt. Owned `key: String` + inferred_borrowed still needs `get(&key)`
            // (P3.576). Do not use `identifier_already_ref` here — it treats all
            // inferred-borrowed WJ `string` params as already-ref.
            let arg_emit_shared_ref = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if gen.emitted_rust_ref_formals.contains(name.as_str())
                        || gen.str_ref_optimized_params.contains(name.as_str())
                        || gen.caller_formal_emitted_shared_ref(name)
            );
            let arg_already_rust_ref = arg_emit_shared_ref;
            let arg_binding_already_shared_ref = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if arg_emit_shared_ref || gen.inferred_borrowed_params.contains(name)
            );
            let coll_sig = type_name.as_deref().and_then(|tn| {
                gen.resolve_method_function_signature(tn, call_method, arguments.len())
            });
            if let Some(ref sig) = coll_sig {
                *arg_str = crate::codegen::rust::call_site_borrow::maybe_borrow_owned_vec_local_for_ref_formal(
                    gen,
                    sig,
                    i,
                    arg_expr,
                    arg_str.clone(),
                    type_name.as_deref(),
                    Some(call_method),
                    Some(arguments.len()),
                );
            }
            crate::codegen::rust::call_site_borrow::finalize_collection_key_call_site_arg(
                coll_sig.as_ref(),
                i,
                arg_expr,
                arg_str,
                arg_already_rust_ref,
                type_name.as_deref(),
                arg_binding_already_shared_ref,
            );
            let map_key_arg = i == 0
                && crate::codegen::rust::stdlib_method_traits::method_is_map_key_qualified(
                    call_method,
                    type_name.as_deref(),
                    &gen.signature_registry,
                );
            if !map_key_arg || arg_emit_shared_ref {
                gen.strip_stale_amp_on_already_ref_arg(arg_expr, arg_str);
            } else if !arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(arg_str);
            }
        }
    }

    // Signature-driven: owned String producers at `&str` / Pattern formals (e.g.
    // `match s.find(":".to_string())`). Parser emits `Call(FieldAccess)` not
    // `MethodCall` for `s.find(...)`, so finalize.rs pattern logic never runs.
    let receiver_is_text = type_name.as_deref().is_some_and(|rt| {
        rt == "String"
            || rt == "string"
            || rt == "str"
            || rt.ends_with("::String")
            || rt.ends_with("::str")
    }) || matches!(
        call_obj,
        Expression::Identifier { name, .. }
            if gen.local_var_types.get(name).is_some_and(|t| {
                crate::codegen::rust::types::is_windjammer_text_type(t)
            })
                || gen.str_ref_optimized_params.contains(name)
                || gen.emitted_rust_ref_formals.contains(name)
                || gen.inferred_borrowed_params.contains(name.as_str())
                || gen.current_function_params.iter().any(|p| {
                    p.name == *name
                        && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                })
    ) || gen
        .infer_expression_type(call_obj)
        .as_ref()
        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
    for (i, arg_str) in args.iter_mut().enumerate() {
        let Some((_, arg_expr)) = arguments.get(i) else {
            continue;
        };
        if crate::codegen::rust::string_utilities::method_call_arg_expects_pattern_str(
            call_method,
            i,
            method_signature.as_ref(),
            type_name.as_deref(),
            receiver_is_text,
            &gen.signature_registry,
        ) && !call_signature_resolution::global_trait_owned_plain_string_arg(
            &gen.signature_registry,
            call_method,
            i,
        ) && !gen.global_signature_registry().is_some_and(|g| {
            call_signature_resolution::global_trait_owned_plain_string_arg(g, call_method, i)
        }) {
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                arg_str,
            );
        }
    }

    let qualified_for_emit = format!("{obj_str}::{call_method}");
    let emit_method = crate::analyzer::SignatureRegistry::resolve_runtime_emit_method_name_chain(
        &qualified_for_emit,
        &gen.signature_registry,
        gen.global_signature_registry(),
    )
    .unwrap_or_else(|| call_method.to_string());

    let call_str = format!(
        "{}{}{}({})",
        obj_str,
        separator,
        emit_method,
        args.join(", ")
    );

    let qualified_name = format!("{}::{}", obj_str, call_method);
    let is_extern_call = method_signature.as_ref().is_some_and(|sig| sig.is_extern)
        || gen.extern_function_names.contains(call_method)
        || gen.extern_function_names.contains(&qualified_name)
        || gen.ffi_module_aliases.contains(&obj_str);

    if is_extern_call && !gen.in_unsafe_block {
        return format!("(unsafe {{ {} }})", call_str);
    }
    call_str
}
