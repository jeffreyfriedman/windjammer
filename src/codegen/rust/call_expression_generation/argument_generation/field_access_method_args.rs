//! Method-style argument strings when dispatch is through `Call(FieldAccess)`.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::super::{expression_helpers, expression_utilities, CodeGenerator};

/// Borrow owned String arguments when the callee's `string` param lowers to `&str`.
fn coerce_string_arg_for_borrowed_callee<'ast>(
    sig: &crate::analyzer::FunctionSignature,
    sig_param_idx: usize,
    arg: &'ast Expression<'ast>,
    mut arg_str: String,
) -> String {
    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
        let param_is_str_ref = matches!(
            param_ty,
            Type::Reference(inner)
                if matches!(**inner, Type::Custom(ref n) if n == "str")
        );
        let is_text_param =
            crate::codegen::rust::types::is_windjammer_text_type(param_ty);
        let callee_borrows_string = !sig.is_extern
            && !arg_str.contains("string_to_ffi(")
            && sig
                .param_ownership
                .get(sig_param_idx)
                .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed))
            || (sig.param_ownership.is_empty() && is_text_param && !sig.is_extern);
        if (param_is_str_ref || callee_borrows_string)
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
        .is_some_and(
            super::super::super::stdlib_method_traits::runtime_std_module_uses_asref_str,
        );
    let arg_is_string = gen.infer_expression_type(arg).as_ref().is_some_and(|t| {
        matches!(t, Type::String)
            || matches!(t, Type::Custom(n) if n == "string" || n == "String")
    });
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
            let prev_coerce_string_literals = gen.coerce_string_literals_to_owned;
            gen.coerce_string_literals_to_owned = false;
            let prev_match_arm_str = gen.in_match_arm_needing_string;
            gen.in_match_arm_needing_string = false;
            let mut arg_str = gen.generate_expression(arg_to_generate);
            gen.coerce_string_literals_to_owned = prev_coerce_string_literals;
            gen.in_match_arm_needing_string = prev_match_arm_str;

            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };

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
                                );
                            if should_ref {
                                arg_str = format!("&{}", arg_str);
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
                        let is_already_mut_ref =
                            if let Expression::Identifier { name, .. } = arg_to_generate {
                                let explicit_mut_ref =
                                    gen.current_function_params.iter().any(|param| {
                                        param.name == *name
                                            && matches!(
                                                &param.type_,
                                                crate::parser::Type::MutableReference(_)
                                            )
                                    });
                                let inferred_mut_ref =
                                    gen.inferred_mut_borrowed_params.contains(name.as_str());
                                explicit_mut_ref || inferred_mut_ref
                            } else {
                                false
                            };
                        if !expression_helpers::is_reference_expression(arg_to_generate)
                            && !is_already_mut_ref
                        {
                            let mut mut_arg_str = if arg_str.ends_with(".clone()") {
                                arg_str[..arg_str.len() - 8].to_string()
                            } else {
                                arg_str
                            };
                            if mut_arg_str.starts_with("&") && !mut_arg_str.starts_with("&mut ") {
                                mut_arg_str = mut_arg_str[1..].to_string();
                            }
                            arg_str = format!("&mut {}", mut_arg_str);
                        }
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
                        if let Expression::FieldAccess {
                            object: field_obj,
                            ..
                        } = arg_to_generate
                        {
                            if let Expression::Identifier { name, .. } = &**field_obj {
                                let is_borrowed = gen.borrowed_iterator_vars.contains(name)
                                    || gen.inferred_borrowed_params.contains(name);
                                if is_borrowed && !arg_str.ends_with(".clone()") {
                                    let is_copy = gen
                                        .infer_expression_type(arg_to_generate)
                                        .as_ref()
                                        .is_some_and(|t| gen.is_type_copy(t));
                                    if !is_copy {
                                        arg_str = format!("{}.clone()", arg_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let qualified_key = type_name
                .as_ref()
                .map(|tn| format!("{}::{}", tn, call_method));
            let has_collision = qualified_key
                .as_ref()
                .is_some_and(|k| gen.signature_registry.has_collision(k))
                || gen.signature_registry.has_collision(call_method);
            if !has_collision {
                if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                    let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                    let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                    if param_is_f32 || param_is_f64 {
                        let arg_ty = gen.infer_expression_type(arg);
                        let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                            matches!(t, Type::Int)
                                || matches!(
                                    t,
                                    Type::Custom(n) if crate::type_classification::is_integer_type(n)
                                )
                        });
                        if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64")
                        {
                            let target = if param_is_f32 { "f32" } else { "f64" };
                            arg_str = if arg_str.contains(' ')
                                || matches!(arg, Expression::Binary { .. })
                            {
                                format!("({}) as {}", arg_str, target)
                            } else {
                                format!("{} as {}", arg_str, target)
                            };
                        }
                    }
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
            let prev_coerce_string_literals = gen.coerce_string_literals_to_owned;
            gen.coerce_string_literals_to_owned = false;
            let prev_match_arm_str = gen.in_match_arm_needing_string;
            gen.in_match_arm_needing_string = false;
            let mut arg_str = gen.generate_expression(arg_to_generate);
            gen.coerce_string_literals_to_owned = prev_coerce_string_literals;
            gen.in_match_arm_needing_string = prev_match_arm_str;

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
                let fb_idx = if fb_sig.has_self_receiver { i + 1 } else { i };
                arg_str = coerce_string_arg_for_borrowed_callee(
                    fb_sig,
                    fb_idx,
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
