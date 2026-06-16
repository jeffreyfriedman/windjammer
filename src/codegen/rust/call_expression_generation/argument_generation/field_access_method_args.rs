//! Method-style argument strings when dispatch is through `Call(FieldAccess)`.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::super::{expression_utilities, CodeGenerator};

/// Borrow owned String arguments when the callee's `string` param lowers to `&str`.
fn coerce_string_arg_for_borrowed_callee<'ast>(
    sig: &crate::analyzer::FunctionSignature,
    sig_param_idx: usize,
    arg: &'ast Expression<'ast>,
    mut arg_str: String,
) -> String {
    use crate::codegen::rust::string_utilities;
    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
        let param_is_str_ref = string_utilities::param_is_rust_str_ref(param_ty);
        let callee_borrows = !arg_str.contains("string_to_ffi(")
            && string_utilities::callee_borrows_string_param(sig, sig_param_idx);
        if (param_is_str_ref || callee_borrows)
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

/// `strings.len(self.text)` etc. — borrow owned fields instead of moving out of `&mut self`.
fn borrow_runtime_std_str_arg<'ast>(
    gen: &CodeGenerator<'ast>,
    runtime_module: Option<&str>,
    type_name: &Option<String>,
    arg: &'ast Expression<'ast>,
    arg_str: String,
) -> String {
    let asref_str_module = runtime_module
        .or_else(|| {
            type_name
                .as_deref()
                .filter(|t| gen.is_imported_runtime_std_module(t))
        })
        .is_some_and(super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str);
    let arg_is_string = gen
        .infer_expression_type(arg)
        .as_ref()
        .is_some_and(crate::codegen::rust::string_utilities::param_is_owned_string_type);
    if asref_str_module
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
    runtime_module: Option<&str>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> Vec<String> {
    arguments
        .iter()
        .enumerate()
        .flat_map(|(i, (_label, arg))| {
            let arg_to_generate =
                expression_utilities::strip_unary_ref_for_collection_key_arg(call_method, i, arg);
            let scope = gen.arg_gen_scope();
            let mut arg_str = gen.generate_expression(arg_to_generate);
            gen.restore_arg_gen_scope(scope);

            let sig_param_idx = sig.arg_param_index(i);

            if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                match ownership {
                    OwnershipMode::Borrowed => {
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

                        let mut string_literal_converted_here = false;

                        if is_string_literal {
                            let param_is_str_ref =
                                sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                    matches!(
                                        t,
                                        Type::Reference(inner)
                                            if matches!(**inner, Type::Custom(ref name) if name == "str")
                                    )
                                });

                            let asref_str_module = runtime_module
                                .or_else(|| {
                                    type_name
                                        .as_deref()
                                        .filter(|t| gen.is_imported_runtime_std_module(t))
                                })
                                .is_some_and(
                                    super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str,
                                );

                            if !param_is_str_ref && !asref_str_module {
                                let param_is_string =
                                    sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                        matches!(t, Type::String)
                                            || matches!(t, Type::Custom(ref name) if name == "string")
                                    });
                                if param_is_string {
                                    arg_str = format!("&{}.to_string()", arg_str);
                                    string_literal_converted_here = true;
                                }
                            }
                        } else if !is_user_closure_param {
                            let param_already_ref = if let Expression::Identifier { name, .. } = arg_to_generate {
                                gen.identifier_already_ref(name)
                            } else { false };
                            if param_already_ref {
                                // str / &string / &T params are already references in Rust — never add &
                            } else {
                            let should_ref =
                                crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
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
                    OwnershipMode::MutBorrowed => {
                        crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                            arg_to_generate,
                            &mut arg_str,
                            &gen.current_function_params,
                            &gen.inferred_mut_borrowed_params,
                        );
                    }
                    OwnershipMode::Owned => {
                        let is_str_lit = matches!(
                            arg_to_generate,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        );
                        let is_str_param = matches!(
                            arg_to_generate,
                            Expression::Identifier { name, .. }
                                if gen.current_function_params.iter().any(|p| {
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
                        if is_str_lit || is_str_param {
                            let asref_str_module = runtime_module
                                .or_else(|| {
                                    type_name
                                        .as_deref()
                                        .filter(|t| gen.is_imported_runtime_std_module(t))
                                })
                                .is_some_and(
                                    super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str,
                                );
                            let is_explicit_str_ref = sig.param_types.get(sig_param_idx).is_some_and(
                                |t| {
                                    matches!(t, Type::Reference(inner) if
                                        matches!(**inner, Type::String) ||
                                        matches!(**inner, Type::Custom(ref s) if s == "str")
                                    )
                                },
                            );
                            if !is_explicit_str_ref && !asref_str_module {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        }
                        gen.maybe_clone_borrowed_field_for_owned_param(arg_to_generate, &mut arg_str);
                    }
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
                runtime_module,
                type_name,
                arg_to_generate,
                arg_str,
            );

            // Borrow owned String/expr args when callee's `string` param is Borrowed (&str in Rust).
            if sig
                .param_ownership
                .get(sig_param_idx)
                .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed))
                && sig.param_types.get(sig_param_idx).is_some_and(
                    crate::codegen::rust::types::is_windjammer_text_type,
                )
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

            arg_str = coerce_string_arg_for_borrowed_callee(
                sig,
                sig_param_idx,
                arg_to_generate,
                arg_str,
            );

            vec![arg_str]
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
pub(in crate::codegen::rust) fn field_access_method_args_fallback<'ast>(
    gen: &mut CodeGenerator<'ast>,
    call_method: &str,
    type_name: &Option<String>,
    runtime_module: Option<&str>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> Vec<String> {
    let qualified_name = type_name
        .as_ref()
        .map(|tn| format!("{}::{}", tn, call_method))
        .unwrap_or_else(|| call_method.to_string());
    let fallback_sig = crate::codegen::rust::call_signature_resolution::resolve_call_signature(
        &gen.signature_registry,
        &qualified_name,
        type_name.as_deref(),
        arguments.len(),
        &gen.module_alias_map,
    )
    .map(|r| r.sig);

    arguments
        .iter()
        .enumerate()
        .map(|(i, (_label, arg))| {
            let arg_to_generate =
                expression_utilities::strip_unary_ref_for_collection_key_arg(call_method, i, arg);
            let scope = gen.arg_gen_scope();
            let mut arg_str = gen.generate_expression(arg_to_generate);
            gen.restore_arg_gen_scope(scope);

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
                let asref_str_module = runtime_module
                    .or_else(|| {
                        type_name
                            .as_deref()
                            .filter(|t| gen.is_imported_runtime_std_module(t))
                    })
                    .is_some_and(
                        super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str,
                    );
                let needs_to_string = !asref_str_module
                    && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(
                        i,
                        call_method,
                        &fallback_sig,
                    );
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
                runtime_module,
                type_name,
                arg_to_generate,
                arg_str,
            );
            if let Some(ref fb_sig) = fallback_sig {
                arg_str = coerce_string_arg_for_borrowed_callee(
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
                let is_instance_call = type_name.is_some();
                if is_instance_call {
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
            arg_str
        })
        .collect()
}
