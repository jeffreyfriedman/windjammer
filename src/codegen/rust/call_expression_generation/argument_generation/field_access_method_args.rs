//! Method-style argument strings when dispatch is through `Call(FieldAccess)`.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::super::{expression_utilities, CodeGenerator};

fn module_qualified_call_name(
    type_name: &Option<String>,
    call_method: &str,
    call_obj: &Expression,
) -> String {
    if let Some(tn) = type_name {
        format!("{tn}::{call_method}")
    } else if let Expression::Identifier { name, .. } = call_obj {
        format!("{name}::{call_method}")
    } else {
        call_method.to_string()
    }
}

fn arg_expression_already_usize<'ast>(
    gen: &CodeGenerator<'ast>,
    arg: &Expression<'ast>,
) -> bool {
    match arg {
        Expression::Identifier { name, .. } => {
            gen.current_function_params
                .iter()
                .find(|p| p.name == *name)
                .is_some_and(|p| matches!(&p.type_, Type::Custom(n) if n == "usize"))
                || gen
                    .local_var_types
                    .get(name)
                    .is_some_and(|t| matches!(t, Type::Custom(n) if n == "usize"))
                || gen.usize_variables.contains(name)
        }
        _ => {
            gen.infer_expression_type_is_usize(arg) || gen.expression_produces_usize(arg)
        }
    }
}

fn coerce_usize_formal_arg<'ast>(
    gen: &CodeGenerator<'ast>,
    sig: &crate::analyzer::FunctionSignature,
    arg_index: usize,
    arg: &Expression<'ast>,
    arg_str: &mut String,
) {
    let sig_param_idx = sig.arg_param_index(arg_index);
    let formal = sig
        .formal_param_type(sig_param_idx)
        .or_else(|| sig.param_types.get(sig_param_idx));
    crate::codegen::rust::type_casting::coerce_arg_str_for_usize_formal(
        arg,
        arg_str,
        formal,
        arg_expression_already_usize(gen, arg),
    );
}

/// Runtime-std WJ-owned / Rust-borrowed slots (`json::get` `&Value`).
/// Signature-driven via `runtime_std_param_needs_auto_borrow_resolved` — not module names.
fn maybe_borrow_runtime_std_wj_owned_rust_borrowed_arg<'ast>(
    gen: &CodeGenerator<'ast>,
    qualified_name: &str,
    arg_index: usize,
    arg: &Expression<'ast>,
    mut arg_str: String,
) -> String {
    if arg_str.starts_with('&') {
        return arg_str;
    }
    if !matches!(
        arg,
        Expression::Identifier { .. } | Expression::FieldAccess { .. }
    ) {
        return arg_str;
    }
    if matches!(
        arg,
        Expression::Identifier { name, .. } if gen.identifier_already_ref(name)
    ) {
        return arg_str;
    }
    let local = gen.signature_registry.get_signature(qualified_name);
    let needs = crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
        &gen.signature_registry,
        qualified_name,
        local,
        arg_index,
    ) || gen.global_signature_registry.as_ref().is_some_and(|g| {
        crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
            g,
            qualified_name,
            g.get_signature(qualified_name).or(local),
            arg_index,
        )
    });
    if needs {
        arg_str.insert(0, '&');
    }
    arg_str
}

fn infer_external_module_mut_borrow_upgrade<'ast>(
    gen: &CodeGenerator<'ast>,
    qualified_name: &str,
    arg_idx: usize,
    arg: &Expression,
    ownership: OwnershipMode,
) -> OwnershipMode {
    if !matches!(ownership, OwnershipMode::Owned) || arg_idx != 0 {
        return ownership;
    }
    if !crate::codegen::rust::call_signature_resolution::is_external_module_qualified_call(
        qualified_name,
    ) {
        return ownership;
    }
    let Expression::Identifier { name, .. } = arg else {
        return ownership;
    };
    if !gen.inferred_mut_borrowed_params.contains(name) {
        return ownership;
    }
    let non_copy = gen
        .current_function_params
        .iter()
        .find(|p| p.name == *name)
        .is_some_and(|p| !gen.is_type_copy(&p.type_))
        || gen
            .infer_expression_type(arg)
            .as_ref()
            .is_none_or(|t| !gen.is_type_copy(t));
    if non_copy {
        OwnershipMode::MutBorrowed
    } else {
        ownership
    }
}

/// Borrow owned String arguments when the callee's `string` param lowers to `&str`.
fn coerce_string_arg_for_borrowed_callee<'ast>(
    gen: &CodeGenerator<'ast>,
    sig: &crate::analyzer::FunctionSignature,
    sig_param_idx: usize,
    arg: &'ast Expression<'ast>,
    mut arg_str: String,
) -> String {
    use crate::codegen::rust::string_utilities;
    if matches!(
        arg,
        Expression::Identifier { name, .. } if gen.identifier_already_ref(name)
            || gen.emitted_rust_ref_formals.contains(name)
            || gen.str_ref_optimized_params.contains(name.as_str())
    ) {
        return arg_str;
    }
    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
        let param_is_str_ref = string_utilities::param_is_rust_str_ref(param_ty);
        let callee_borrows = !arg_str.contains("string_to_ffi(")
            && string_utilities::callee_borrows_string_param(sig, sig_param_idx);
        let arg_is_text_compatible = gen
            .infer_expression_type(arg)
            .as_ref()
            .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
        if (param_is_str_ref || callee_borrows)
            && arg_is_text_compatible
            && !arg_expression_is_copy_non_text_scalar(gen, arg)
            && !arg_str.starts_with('&')
            && !matches!(
                arg,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            )
        {
            arg_str = format!("&{arg_str}");
        }
    }
    arg_str
}

fn arg_expression_is_copy_non_text_scalar<'ast>(
    gen: &CodeGenerator<'ast>,
    arg: &'ast Expression<'ast>,
) -> bool {
    if let Some(t) = gen.infer_expression_type(arg) {
        return gen.is_type_copy(&t) && !crate::codegen::rust::types::is_windjammer_text_type(&t);
    }
    if let Expression::Identifier { name, .. } = arg {
        if let Some(param) = gen.current_function_params.iter().find(|p| p.name == *name) {
            return gen.is_type_copy(&param.type_)
                && !crate::codegen::rust::types::is_windjammer_text_type(&param.type_);
        }
    }
    false
}

/// Borrow owned string fields/locals when the resolved formal is WJ-owned + Rust-borrowed.
fn borrow_runtime_std_str_arg<'ast>(
    gen: &CodeGenerator<'ast>,
    qualified_name: &str,
    sig: Option<&crate::analyzer::FunctionSignature>,
    arg_index: usize,
    arg: &'ast Expression<'ast>,
    arg_str: String,
) -> String {
    let arg_is_string =
        crate::codegen::rust::string_utilities::expression_is_owned_string_for_asref_borrow(
            arg,
            gen.infer_expression_type(arg).as_ref(),
            &gen.local_var_types,
            &gen.current_function_params,
        );
    let needs_borrow = crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
        &gen.signature_registry,
        qualified_name,
        sig,
        arg_index,
    ) || sig.is_some_and(|s| {
        crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(s, arg_index)
    });
    if needs_borrow
        && arg_is_string
        && matches!(
            arg,
            Expression::FieldAccess { .. } | Expression::Identifier { .. }
        )
        && !arg_str.starts_with('&')
        && !arg_str.ends_with(".clone()")
    {
        format!("&{arg_str}")
    } else {
        arg_str
    }
}

#[allow(clippy::too_many_lines)]
pub(in crate::codegen::rust) fn field_access_method_args_with_signature<'ast>(
    gen: &mut CodeGenerator<'ast>,
    sig: &crate::analyzer::FunctionSignature,
    call_method: &str,
    method_signature: &Option<crate::analyzer::FunctionSignature>,
    type_name: &Option<String>,
    call_obj: &Expression<'ast>,
    runtime_module: Option<&str>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> Vec<String> {
    let qualified_name = module_qualified_call_name(type_name, call_method, call_obj);
    let runtime_module = runtime_module.or_else(|| {
        qualified_name.split("::").next().filter(|m| {
            crate::codegen::rust::stdlib_method_traits::is_runtime_std_module(m)
        })
    });
    arguments
        .iter()
        .enumerate()
        .flat_map(|(i, (_label, arg))| {
            let arg_to_generate = expression_utilities::strip_unary_ref_for_collection_key_arg(
                i,
                arg,
                Some(sig),
                type_name.as_deref(),
            );
            let scope = gen.arg_gen_scope();
            let mut arg_str = gen.generate_expression(arg_to_generate);
            gen.restore_arg_gen_scope(scope);
            arg_str =
                gen.peel_copy_ref_match_binding_for_value(arg_to_generate, &arg_str);

            if gen.ir_cutover.call_sites {
                if let Some(mut coerced) = gen.apply_ir_call_site_coercion(
                    &gen.signature_registry,
                    &qualified_name,
                    i,
                    arg_to_generate,
                    &arg_str,
                    Some(sig),
                    type_name.as_deref(),
                    Some(arguments.len()),
                ) {
                    let effective_sig = type_name
                        .as_ref()
                        .and_then(|tn| {
                            gen.resolve_method_function_signature(
                                tn,
                                call_method,
                                arguments.len(),
                            )
                        })
                        .unwrap_or_else(|| sig.clone());
                    gen.reconcile_post_ir_mut_borrow_and_owned_peel(
                        &mut coerced,
                        arg_to_generate,
                        &qualified_name,
                        i,
                        &effective_sig,
                        &gen.signature_registry,
                        type_name.as_deref(),
                        Some(call_obj),
                        Some(arguments.len()),
                        false,
                    );
                    coerce_usize_formal_arg(gen, sig, i, arg_to_generate, &mut coerced);
                    return vec![coerced];
                }
            }

            let sig_param_idx = sig.arg_param_index(i);

            // Check for ownership collision on this method — skip auto-borrow if
            // the registry has conflicting ownership for the same function name.
            let has_method_collision = gen.has_collision_with_global(call_method)
                || gen.has_collision_with_global(&qualified_name);

            let ownership = if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                sig,
                sig_param_idx,
            ) {
                OwnershipMode::Owned
            } else {
                infer_external_module_mut_borrow_upgrade(
                    gen,
                    &qualified_name,
                    i,
                    arg_to_generate,
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                        sig,
                        i,
                        type_name.as_deref(),
                    ),
                )
            };

            let callee_formal_is_copy = sig
                .formal_param_type(sig_param_idx)
                .is_some_and(|t| gen.is_type_copy(t));
            // Copy aggregates that emit `&mut T` still need mut-borrow at the call site.
            let apply_mut_despite_copy = callee_formal_is_copy
                && (matches!(
                    sig.param_types.get(sig_param_idx),
                    Some(Type::MutableReference(_))
                ) || crate::codegen::rust::call_signature_resolution::callee_user_arg_expects_mut_borrow(
                    sig, i,
                ));

            match ownership {
                    OwnershipMode::Borrowed if !has_method_collision && !callee_formal_is_copy => {
                        let is_string_literal = matches!(
                            arg_to_generate,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        );
                        let is_user_closure_param =
                            if let Expression::Identifier { name, .. } = arg_to_generate {
                                gen.in_user_written_closure && gen.user_closure_params.contains(name)
                            } else {
                                false
                            };

                        let string_literal_converted_here = false;

                        if is_string_literal {
                            // Borrowed text formals accept bare `"lit"` as `&str` — never
                            // allocate `&"lit".to_string()` here.
                            let _ = string_literal_converted_here;
                        } else if !is_user_closure_param {
                            let param_already_ref = if let Expression::Identifier { name, .. } = arg_to_generate {
                                gen.identifier_already_ref(name)
                            } else { false };
                            if param_already_ref {
                                // str / &string / &T params are already references in Rust — never add &
                            } else {
                            let should_ref =
                                !arg_expression_is_copy_non_text_scalar(gen, arg_to_generate)
                                    && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                    arg_to_generate,
                                    &arg_str,
                                    call_method,
                                    i,
                                    method_signature,
                                    &gen.usize_variables,
                                    &gen.current_function_params,
                                    &gen.borrowed_iterator_vars,
                                    &gen.inferred_borrowed_params,
                                    arguments.len(),
                                    type_name.as_deref(),
                                    Some(&gen.local_var_types),
                                    Some(&gen.stdlib_method_signatures),
                                    Some(&gen.method_signatures_by_type),
                                    &gen.match_arm_bindings,
                                    &gen.str_ref_optimized_params,
                                    Some(&gen.signature_registry),
                                );
                            if should_ref {
                                arg_str = format!("&{}", arg_str);
                            }
                            }
                        }

                        arg_str = gen.ensure_ref_for_owned_string_field_when_callee_expects_str(
                            method_signature,
                            sig_param_idx,
                            arg_to_generate,
                            arg_str,
                            string_literal_converted_here,
                        );
                    }
                    OwnershipMode::MutBorrowed
                        if !has_method_collision
                            && (!callee_formal_is_copy || apply_mut_despite_copy) =>
                    {
                        crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                            arg_to_generate,
                            &mut arg_str,
                            &gen.current_function_params,
                            &gen.inferred_mut_borrowed_params,
                        );
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);
                    }
                    OwnershipMode::Owned => {
                        let is_str_lit = matches!(
                            arg_to_generate,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        );
                        let is_caller_str_slice_param = matches!(
                            arg_to_generate,
                            Expression::Identifier { name, .. }
                                if gen.str_ref_optimized_params.contains(name.as_str())
                                    || gen.inferred_borrowed_params.contains(name)
                                    || gen.current_function_params.iter().any(|p| {
                                        &p.name == name
                                            && matches!(
                                                &p.type_,
                                                Type::Reference(inner)
                                                    if matches!(
                                                        **inner,
                                                        Type::Custom(ref s) if s == "str"
                                                    )
                                            )
                                    })
                        );
                        let arg_is_owned_string_binding =
                            if let Expression::Identifier { name, .. } = arg_to_generate {
                                gen.current_function_params.iter().any(|p| {
                                    p.name == *name
                                        && crate::codegen::rust::types::is_windjammer_text_type(
                                            &p.type_,
                                        )
                                        && !matches!(
                                            &p.type_,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                })
                            } else {
                                false
                            };
                        if (is_str_lit || is_caller_str_slice_param) && !arg_is_owned_string_binding
                        {
                            let skips_owned_literal =
                                crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
                                    Some(sig),
                                    i,
                                );
                            let static_impl_borrows =
                                crate::codegen::rust::call_signature_resolution::static_impl_text_borrows_at_call_site(
                                    sig,
                                    sig_param_idx,
                                );
                            let needs_owned_literal =
                                crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion_with_enum(
                                    Some(sig),
                                    i,
                                    Some(call_method),
                                    type_name.as_deref(),
                                    Some(&gen.enum_variant_types),
                                    runtime_module,
                                )
                                || (is_str_lit
                                    && crate::codegen::rust::string_utilities::type_qualified_associated_string_literal_needs_rust_owned_string(
                                        &qualified_name,
                                        i,
                                        Some(sig),
                                        &gen.signature_registry,
                                        gen.global_signature_registry.as_deref(),
                                    ));
                            let is_explicit_str_ref = sig
                                .formal_param_type(sig_param_idx)
                                .or_else(|| sig.param_types.get(sig_param_idx))
                                .is_some_and(|t| {
                                    matches!(t, Type::Reference(inner) if
                                        matches!(**inner, Type::String) ||
                                        matches!(**inner, Type::Custom(ref s) if s == "str")
                                    )
                                });
                            let needs_owned_from_str_param = is_caller_str_slice_param
                                && !is_explicit_str_ref
                                && !skips_owned_literal
                                && !static_impl_borrows;
                            if (!is_explicit_str_ref
                                && !skips_owned_literal
                                && !static_impl_borrows
                                && needs_owned_literal)
                                || needs_owned_from_str_param
                            {
                                if !arg_str.ends_with(".to_string()") {
                                    arg_str = format!("{}.to_string()", arg_str);
                                }
                            }
                        }
                        gen.maybe_clone_borrowed_field_for_owned_param(arg_to_generate, &mut arg_str);
                        if let Expression::Identifier { name, .. } = arg_to_generate {
                            if (gen.identifier_already_ref(name)
                                || gen.emitted_rust_ref_formals.contains(name))
                                && !callee_formal_is_copy
                                && !arg_str.ends_with(".clone()")
                            {
                                arg_str = format!(
                                    "{}.clone()",
                                    arg_str.trim_start_matches('&')
                                );
                            }
                        }
                    }
                    _ => {
                        // Collision guard triggered for Borrowed/MutBorrowed.
                        // Pass argument as-is; don't apply auto-borrow.
                    }
                }

            // AUTO-CAST int → float
            let qualified_key = type_name
                .as_ref()
                .map(|tn| format!("{}::{}", tn, call_method));
            let skip_cast = gen.should_skip_int_to_float_auto_cast_with_global(
                type_name.as_deref(),
                call_method,
                qualified_key.as_deref(),
            );
            if !skip_cast {
                if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                    let arg_ty = gen.infer_expression_type(arg);
                    crate::codegen::rust::type_classification_utilities::maybe_cast_int_arg_to_float(
                        &mut arg_str, arg, param_ty, arg_ty.as_ref(),
                    );
                }
            }

            arg_str = borrow_runtime_std_str_arg(
                gen,
                &qualified_name,
                Some(sig),
                i,
                arg_to_generate,
                arg_str,
            );

            // Borrow owned String/expr args when callee's `string` param is Borrowed (&str in Rust).
            // Skip when ownership collision detected for this method.
            // Skip when the arg binding is already a shared-ref formal (`key: &str`).
            let arg_already_shared_ref = matches!(
                arg_to_generate,
                Expression::Identifier { name, .. }
                    if gen.identifier_already_ref(name)
                        || gen.emitted_rust_ref_formals.contains(name)
                        || gen.str_ref_optimized_params.contains(name.as_str())
            );
            if !has_method_collision
                && !arg_already_shared_ref
                && matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                        sig, sig_param_idx,
                    ),
                    OwnershipMode::Borrowed
                ) && sig.param_types.get(sig_param_idx).is_some_and(
                    crate::codegen::rust::types::is_windjammer_text_type,
                )
                && gen
                    .infer_expression_type(arg_to_generate)
                    .as_ref()
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                && !arg_expression_is_copy_non_text_scalar(gen, arg_to_generate)
                && !arg_str.starts_with('&')
                && matches!(
                    arg_to_generate,
                    Expression::Identifier { .. }
                        | Expression::FieldAccess { .. }
                        | Expression::MethodCall { .. }
                )
            {
                arg_str = format!("&{arg_str}");
            }

            if !has_method_collision {
                arg_str = coerce_string_arg_for_borrowed_callee(
                    gen,
                    sig,
                    sig_param_idx,
                    arg_to_generate,
                    arg_str,
                );
            }

            crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
                Some(sig),
                i,
                Some(call_method),
                arg_to_generate,
                &mut arg_str,
                type_name.as_deref(),
                Some(&gen.enum_variant_types),
                runtime_module,
            );

            let arg_already_rust_ref = matches!(
                arg_to_generate,
                Expression::Identifier { name, .. }
                    if gen.identifier_already_ref(name)
                        || gen.str_ref_optimized_params.contains(name.as_str())
                        || gen.inferred_borrowed_params.contains(name)
            );
            crate::codegen::rust::call_site_borrow::finalize_collection_key_call_site_arg(
                Some(sig),
                i,
                arg_to_generate,
                &mut arg_str,
                arg_already_rust_ref,
                type_name.as_deref(),
                false,
            );

            arg_str = maybe_borrow_runtime_std_wj_owned_rust_borrowed_arg(
                gen,
                &qualified_name,
                i,
                arg_to_generate,
                arg_str,
            );

            gen.strip_stale_amp_on_already_ref_arg(arg_to_generate, &mut arg_str);

            coerce_usize_formal_arg(gen, sig, i, arg_to_generate, &mut arg_str);

            vec![arg_str]
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
pub(in crate::codegen::rust) fn field_access_method_args_fallback<'ast>(
    gen: &mut CodeGenerator<'ast>,
    call_method: &str,
    type_name: &Option<String>,
    call_obj: &Expression<'ast>,
    runtime_module: Option<&str>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> Vec<String> {
    let qualified_name = module_qualified_call_name(type_name, call_method, call_obj);
    let runtime_module = runtime_module.or_else(|| {
        qualified_name.split("::").next().filter(|m| {
            crate::codegen::rust::stdlib_method_traits::is_runtime_std_module(m)
        })
    });
    let fallback_sig = type_name
        .as_ref()
        .and_then(|tn| {
            gen.lookup_method_signature_on_receiver_type(tn, call_method, arguments.len())
        })
        .or_else(|| {
            gen.resolve_call_signature_with_global(
                &qualified_name,
                type_name.as_deref(),
                arguments.len(),
            )
            .filter(|r| {
                match r.resolution_method {
                    crate::codegen::rust::call_signature_resolution::ResolutionMethod::ArgCountValidated => {
                        type_name.as_ref().is_some_and(|tn| {
                            crate::codegen::rust::call_signature_resolution::arg_count_validated_matches_receiver(
                                &r.qualified_key,
                                tn,
                                call_method,
                            )
                        })
                    }
                    _ => true,
                }
            })
            .map(|r| r.sig)
        });

    arguments
        .iter()
        .enumerate()
        .map(|(i, (_label, arg))| {
            let arg_to_generate = expression_utilities::strip_unary_ref_for_collection_key_arg(
                i,
                arg,
                fallback_sig.as_ref(),
                type_name.as_deref(),
            );
            let scope = gen.arg_gen_scope();
            let mut arg_str = gen.generate_expression(arg_to_generate);
            gen.restore_arg_gen_scope(scope);
            arg_str =
                gen.peel_copy_ref_match_binding_for_value(arg_to_generate, &arg_str);

            // Phase 5: IR owns coercion when call_sites is on — return early like
            // `field_access_method_args_with_signature` (no legacy double-patch).
            if gen.ir_cutover.call_sites {
                if let Some(mut coerced) = gen.apply_ir_call_site_coercion(
                    &gen.signature_registry,
                    &qualified_name,
                    i,
                    arg_to_generate,
                    &arg_str,
                    fallback_sig.as_ref(),
                    type_name.as_deref(),
                    None,
                ) {
                    if let Some(ref sig) = fallback_sig {
                        gen.reconcile_post_ir_mut_borrow_and_owned_peel(
                            &mut coerced,
                            arg_to_generate,
                            &qualified_name,
                            i,
                            sig,
                            &gen.signature_registry,
                            type_name.as_deref(),
                            Some(call_obj),
                            Some(arguments.len()),
                            false,
                        );
                        coerce_usize_formal_arg(gen, sig, i, arg_to_generate, &mut coerced);
                    }
                    return coerced;
                }
                debug_assert!(
                    false,
                    "IR call-site coercion must be total when call_sites is on ({qualified_name})"
                );
                // Phase 5: never fall through to legacy field-access ownership path.
                return arg_str;
            }

            arg_str = maybe_borrow_runtime_std_wj_owned_rust_borrowed_arg(
                gen,
                &qualified_name,
                i,
                arg_to_generate,
                arg_str,
            );

            let is_string_literal = matches!(
                arg_to_generate,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            );
            let is_str_param = matches!(
                arg_to_generate,
                Expression::Identifier { name, .. }
                    if gen.inferred_borrowed_params.contains(name)
                        || gen.current_function_params.iter().any(|p| {
                            &p.name == name
                                && matches!(
                                    &p.type_,
                                    Type::Reference(inner)
                                        if matches!(**inner, Type::Custom(ref s) if s == "str")
                                )
                        })
            );
            if is_string_literal || is_str_param {
                let skips_owned_literal =
                    crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
                        fallback_sig.as_ref(),
                        i,
                    );
                let needs_to_string = !skips_owned_literal
                    && !gen.inline_module_qualified_call(&qualified_name)
                    && !crate::codegen::rust::call_signature_resolution::is_lowercase_user_module_qualified_call(
                        &qualified_name,
                    )
                    && (fallback_sig.as_ref().is_some_and(|sig| {
                        crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion_with_enum(
                            Some(sig),
                            i,
                            Some(call_method),
                            type_name.as_deref(),
                            Some(&gen.enum_variant_types),
                            runtime_module,
                        )
                    })
                    || (is_string_literal
                        && crate::codegen::rust::string_utilities::type_qualified_associated_string_literal_needs_rust_owned_string(
                            &qualified_name,
                            i,
                            fallback_sig.as_ref(),
                            &gen.signature_registry,
                            gen.global_signature_registry.as_deref(),
                        )));
                if needs_to_string {
                    arg_str = format!("{}.to_string()", arg_str);
                }
            }

            let should_ref =
                crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                    arg_to_generate,
                    &arg_str,
                    call_method,
                    i,
                    &fallback_sig,
                    &gen.usize_variables,
                    &gen.current_function_params,
                    &gen.borrowed_iterator_vars,
                    &gen.inferred_borrowed_params,
                    arguments.len(),
                    type_name.as_deref(),
                    Some(&gen.local_var_types),
                    Some(&gen.stdlib_method_signatures),
                    Some(&gen.method_signatures_by_type),
                    &gen.match_arm_bindings,
                    &gen.str_ref_optimized_params,
                    Some(&gen.signature_registry),
                );
            if should_ref {
                arg_str = format!("&{}", arg_str);
            }

            let string_literal_converted_here =
                (is_string_literal || is_str_param) && arg_str.ends_with(".to_string()");
            if let Some(fb_idx) = fallback_sig.as_ref().map(|s| {
                if s.has_self_receiver {
                    i + 1
                } else {
                    i
                }
            }) {
                arg_str = gen.ensure_ref_for_owned_string_field_when_callee_expects_str(
                    &fallback_sig,
                    fb_idx,
                    arg_to_generate,
                    arg_str,
                    string_literal_converted_here,
                );
            }
            arg_str = borrow_runtime_std_str_arg(
                gen,
                &qualified_name,
                fallback_sig.as_ref(),
                i,
                arg_to_generate,
                arg_str,
            );
            if let Some(ref fb_sig) = fallback_sig {
                arg_str = coerce_string_arg_for_borrowed_callee(
                    gen,
                    fb_sig,
                    fb_sig.arg_param_index(i),
                    arg_to_generate,
                    arg_str,
                );
            } else {
                // No signature: only borrow when type_name is known (instance
                // call on a resolved receiver). Without a signature we can't
                // tell if the callee takes owned or borrowed, so being
                // conservative and skipping the borrow for unknown methods
                // avoids incorrect `&` on owned params (e.g. Vec::push).
                //
                // Static associated calls (`VoxelScene::new(64)`) also infer a
                // CamelCase type_name — do not treat those as instance methods.
                let is_copy_literal = matches!(
                    arg_to_generate,
                    Expression::Literal {
                        value: Literal::Int(_)
                            | Literal::IntSuffixed(_, _)
                            | Literal::Float(_)
                            | Literal::Bool(_),
                        ..
                    }
                );
                let is_static_associated_call = type_name.as_ref().is_some_and(|tn| {
                    tn.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                });
                let is_instance_call =
                    type_name.is_some() && !is_static_associated_call;
                if is_instance_call && !is_copy_literal {
                    let is_non_copy_value = gen
                        .infer_expression_type(arg_to_generate)
                        .as_ref()
                        .is_none_or(|t| !gen.is_type_copy(t));
                    if is_non_copy_value
                        && !arg_str.starts_with('&')
                        && !matches!(
                            arg_to_generate,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        )
                    {
                        arg_str = format!("&{arg_str}");
                    }
                }
            }
            crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
                fallback_sig.as_ref(),
                i,
                Some(call_method),
                arg_to_generate,
                &mut arg_str,
                type_name.as_deref(),
                Some(&gen.enum_variant_types),
                runtime_module,
            );
            if i == 0 {
                let upgraded = infer_external_module_mut_borrow_upgrade(
                    gen,
                    &qualified_name,
                    i,
                    arg_to_generate,
                    OwnershipMode::Owned,
                );
                if matches!(upgraded, OwnershipMode::MutBorrowed) {
                    crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                        arg_to_generate,
                        &mut arg_str,
                        &gen.current_function_params,
                        &gen.inferred_mut_borrowed_params,
                    );
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);
                }
            }
            let arg_already_rust_ref = matches!(
                arg_to_generate,
                Expression::Identifier { name, .. }
                    if gen.identifier_already_ref(name)
                        || gen.str_ref_optimized_params.contains(name.as_str())
                        || gen.inferred_borrowed_params.contains(name)
            );
            crate::codegen::rust::call_site_borrow::finalize_collection_key_call_site_arg(
                fallback_sig.as_ref(),
                i,
                arg_to_generate,
                &mut arg_str,
                arg_already_rust_ref,
                type_name.as_deref(),
                false,
            );
            gen.strip_stale_amp_on_already_ref_arg(arg_to_generate, &mut arg_str);

            if let Some(ref sig) = fallback_sig {
                coerce_usize_formal_arg(gen, sig, i, arg_to_generate, &mut arg_str);
            }

            arg_str
        })
        .collect()
}
