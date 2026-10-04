//! Method-style argument strings when dispatch is through `Call(FieldAccess)`.

use crate::parser::*;

use super::super::super::{expression_utilities, CodeGenerator};

/// Callee key for IR call-site coercion on `Call(FieldAccess)` dispatch.
///
/// Must not invent `local_var::method` from lowercase identifiers — that falsely
/// trips module-boundary fail-closed (`missing boundary signature for after::find`).
/// Delegate to the shared Type/module-qualified helper used by MethodCall lowering.
fn module_qualified_call_name(
    type_name: &Option<String>,
    call_method: &str,
    call_obj: &Expression,
    is_imported_runtime_std_module: impl Fn(&str) -> bool,
) -> String {
    crate::codegen::rust::stdlib_method_traits::module_qualified_method_name(
        type_name.as_deref(),
        call_obj,
        call_method,
        is_imported_runtime_std_module,
    )
}

/// `self.field` behind `&self`/`&mut self` into an owned formal must `.clone()`.
fn ensure_clone_borrowed_self_field_into_owned(
    gen: &CodeGenerator<'_>,
    coerced: &mut String,
    arg: &Expression<'_>,
    sig: &crate::analyzer::FunctionSignature,
    arg_index: usize,
    qualified_name: &str,
) {
    if !matches!(arg, Expression::FieldAccess { .. } | Expression::Index { .. }) {
        return;
    }
    if coerced.ends_with(".clone()") || coerced.ends_with(".to_owned()") {
        return;
    }
    if gen.expression_is_copy(arg) {
        return;
    }
    if !gen.field_access_root_is_behind_reference(arg) {
        return;
    }
    let pidx = sig.arg_param_index(arg_index);
    let owned_slot = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
        || crate::analyzer::SignatureRegistry::stdlib()
            .get_signature(qualified_name)
            .or_else(|| {
                let simple = qualified_name.rsplit("::").next().unwrap_or(qualified_name);
                crate::analyzer::SignatureRegistry::stdlib().get_signature(simple)
            })
            .is_some_and(|std_sig| {
                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    std_sig,
                    std_sig.arg_param_index(arg_index),
                )
            });
    if !owned_slot {
        return;
    }
    let base = if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
        coerced[1..].to_string()
    } else {
        coerced.clone()
    };
    *coerced = format!("{base}.clone()");
}

pub(in crate::codegen::rust) fn field_access_method_args_with_signature<'ast>(
    gen: &mut CodeGenerator<'ast>,
    sig: &crate::analyzer::FunctionSignature,
    call_method: &str,
    _method_signature: &Option<crate::analyzer::FunctionSignature>,
    type_name: &Option<String>,
    call_obj: &Expression<'ast>,
    _runtime_module: Option<&str>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> Vec<String> {
    let mut refreshed = sig.clone();
    let mut keys: Vec<String> = Vec::new();
    if let Some(tn) = type_name.as_deref() {
        keys.push(format!("{tn}::{call_method}"));
    }
    if !refreshed.name.is_empty() {
        keys.push(refreshed.name.clone());
    }
    if keys.is_empty() {
        keys.push(call_method.to_string());
    }
    crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
        &mut refreshed,
        &gen.signature_registry,
        &keys,
    );
    if let Some(global) = gen.global_signature_registry.as_ref() {
        crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
            &mut refreshed,
            global,
            &keys,
        );
    }
    let sig = &refreshed;
    let qualified_name = module_qualified_call_name(type_name, call_method, call_obj, |name| {
        gen.is_imported_runtime_std_module(name)
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
            arg_str = gen.peel_copy_ref_match_binding_for_value(arg_to_generate, &arg_str);

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
                            gen.resolve_method_function_signature(tn, call_method, arguments.len())
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
                    // P3.576: match scrutinees parse as Call(FieldAccess); MethodCall's
                    // collection-key finalize must run here too (`g.get(&key)`).
                    gen.finalize_post_ir_collection_key_arg(
                        &mut coerced,
                        arg_to_generate,
                        i,
                        call_method,
                        &qualified_name,
                        type_name.as_deref(),
                        &effective_sig,
                    );
                    crate::codegen::rust::call_site_borrow::reconcile_method_call_owned_copy_scalar_identifier_arg(
                        gen,
                        &mut coerced,
                        arg_to_generate,
                        &effective_sig,
                        i,
                        type_name.as_deref(),
                    );
                    // P3.636: `json.to_string(self.events)` is Call(FieldAccess), not a
                    // free `json::to_string` site � clone `self.field` behind `&mut self`
                    // into owned formals when IR missed Clone.
                    ensure_clone_borrowed_self_field_into_owned(
                        gen,
                        &mut coerced,
                        arg_to_generate,
                        &effective_sig,
                        i,
                        &qualified_name,
                    );
                    return vec![coerced];
                }
                debug_assert!(
                    false,
                    "IR call-site coercion must be total when call_sites is on ({qualified_name})"
                );
                // P3.635 / WDB-134: IR miss must not skip HashMap key borrow on match
                // scrutinees (`self.inner.get(label)`).
                let effective_sig = type_name
                    .as_ref()
                    .and_then(|tn| {
                        gen.resolve_method_function_signature(tn, call_method, arguments.len())
                    })
                    .unwrap_or_else(|| sig.clone());
                let mut out = arg_str;
                gen.finalize_post_ir_collection_key_arg(
                    &mut out,
                    arg_to_generate,
                    i,
                    call_method,
                    &qualified_name,
                    type_name.as_deref(),
                    &effective_sig,
                );
                return vec![out];
            }

            vec![arg_str]
        })
        .collect()
}

pub(in crate::codegen::rust) fn field_access_method_args_fallback<'ast>(
    gen: &mut CodeGenerator<'ast>,
    call_method: &str,
    type_name: &Option<String>,
    call_obj: &Expression<'ast>,
    _runtime_module: Option<&str>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> Vec<String> {
    let qualified_name = module_qualified_call_name(type_name, call_method, call_obj, |name| {
        gen.is_imported_runtime_std_module(name)
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
            arg_str = gen.peel_copy_ref_match_binding_for_value(arg_to_generate, &arg_str);

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
                        gen.finalize_post_ir_collection_key_arg(
                            &mut coerced,
                            arg_to_generate,
                            i,
                            call_method,
                            &qualified_name,
                            type_name.as_deref(),
                            sig,
                        );
                        crate::codegen::rust::call_site_borrow::reconcile_method_call_owned_copy_scalar_identifier_arg(
                            gen,
                            &mut coerced,
                            arg_to_generate,
                            sig,
                            i,
                            type_name.as_deref(),
                        );
                    } else if let Some(std_sig) = crate::codegen::rust::stdlib_method_traits::hashmap_key_method_signature_for_wrapper(
                        call_method,
                        type_name.as_deref(),
                        &gen.signature_registry,
                    ) {
                        gen.finalize_post_ir_collection_key_arg(
                            &mut coerced,
                            arg_to_generate,
                            i,
                            call_method,
                            &qualified_name,
                            type_name.as_deref(),
                            &std_sig,
                        );
                    }
                    if let Some(ref sig) = fallback_sig {
                        ensure_clone_borrowed_self_field_into_owned(
                            gen,
                            &mut coerced,
                            arg_to_generate,
                            sig,
                            i,
                            &qualified_name,
                        );
                    } else if let Some(std_sig) =
                        crate::analyzer::SignatureRegistry::stdlib().get_signature(&qualified_name)
                    {
                        ensure_clone_borrowed_self_field_into_owned(
                            gen,
                            &mut coerced,
                            arg_to_generate,
                            std_sig,
                            i,
                            &qualified_name,
                        );
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

            arg_str
        })
        .collect()
}
