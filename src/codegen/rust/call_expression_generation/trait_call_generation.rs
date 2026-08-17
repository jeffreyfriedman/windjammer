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
    let type_name = gen.infer_type_name(call_obj);
    let registry = gen
        .global_signature_registry()
        .unwrap_or(&gen.signature_registry);
    let is_type_preserving =
        crate::codegen::rust::stdlib_method_traits::method_is_type_preserving_qualified(
            call_method,
            type_name.as_deref(),
            registry,
        );

    let prev_explicit_clone = gen.in_explicit_clone_call;
    if is_type_preserving {
        gen.in_explicit_clone_call = true;
    }
    let mut obj_str = gen.generate_expression(call_obj);
    gen.in_explicit_clone_call = prev_explicit_clone;

    // Strip redundant auto-clone before an explicit type-preserving `.clone()`.
    if call_method == "clone" && obj_str.ends_with(".clone()") {
        obj_str = obj_str[..obj_str.len() - 8].to_string();
    }

    // Prefer converged signature_registry (Owned consumers like MannequinMesh::generate)
    // over per-body method_signatures_by_type (may infer Borrowed when a formal is reused
    // inside the callee). Method registry remains fallback for stdlib / not-yet-registered.
    let from_registry = type_name.as_ref().and_then(|tn| {
        call_signature_resolution::resolve_method_for_call_site(
            &gen.signature_registry,
            gen.global_signature_registry(),
            tn,
            call_method,
            arguments.len(),
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
        let registry = gen
            .global_signature_registry()
            .unwrap_or(&gen.signature_registry);
        // Prefer runtime scanner fallback (`&str`) over WJ-owned stubs for the same key.
        let baseline = registry
            .get_fallback_signature(&key)
            .or_else(|| registry.get_signature(&key))?;
        let mut sig = baseline.clone();
        if let Some(local) = registry.get_signature(&key) {
            if let Some(preferred) =
                crate::codegen::rust::signature_promotion::prefer_shared_ref_signature(
                    Some(local.clone()),
                    Some(&sig),
                    0,
                )
            {
                sig = preferred;
            }
        }
        Some(call_signature_resolution::finalize_call_site_signature(sig))
    });

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
            let arg_already_rust_ref = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if gen.identifier_already_ref(name)
                        || gen.str_ref_optimized_params.contains(name.as_str())
                        || gen.inferred_borrowed_params.contains(name)
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
                false,
            );
            gen.strip_stale_amp_on_already_ref_arg(arg_expr, arg_str);
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
        ) {
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                arg_str,
            );
        }
    }

    let call_str = format!(
        "{}{}{}({})",
        obj_str,
        separator,
        call_method,
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
