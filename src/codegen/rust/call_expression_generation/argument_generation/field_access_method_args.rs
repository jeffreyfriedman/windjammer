//! Method-style argument strings when dispatch is through `Call(FieldAccess)`.

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
    let qualified_name = module_qualified_call_name(type_name, call_method, call_obj);
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
                    return vec![coerced];
                }
                debug_assert!(
                    false,
                    "IR call-site coercion must be total when call_sites is on ({qualified_name})"
                );
                return vec![arg_str];
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
    let qualified_name = module_qualified_call_name(type_name, call_method, call_obj);
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
