//! Method-call argument codegen.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use crate::codegen::rust::CodeGenerator;

/// Rust stdlib methods whose closure parameter receives `&T` (not owned `T`).
/// For these methods, closure params are references and comparisons need deref.
fn is_ref_closure_method(method: &str) -> bool {
    matches!(
        method,
        "retain"
            | "filter"
            | "any"
            | "all"
            | "find"
            | "position"
            | "rposition"
            | "take_while"
            | "skip_while"
            | "partition"
            | "inspect"
    )
}

impl<'ast> CodeGenerator<'ast> {
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn mc_build_method_call_arg_strings(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        type_name: Option<String>,
    ) -> (Vec<String>, Option<Type>) {
        self.refresh_pure_forwarding_delegate_flag();
        // Float method argument context: for methods like clamp/max/min on float
        // receivers, arguments should use the same float type as the receiver.
        let prev_float_target = self.assignment_float_target_type.clone();
        let receiver_type_inferred = self.infer_expression_type(object);
        let receiver_is_string_collection =
            Self::is_string_element_collection(receiver_type_inferred.as_ref())
                || match object {
                    Expression::Identifier { name, .. } => {
                        Self::is_string_element_collection(self.local_var_types.get(name.as_str()))
                    }
                    _ => false,
                }
                || Self::is_string_collection_from_return_type(
                    receiver_type_inferred.as_ref(),
                    self.current_function_return_type.as_ref(),
                );
        let is_float_method = crate::type_classification::is_float_receiver_method(method);
        if is_float_method {
            if let Some(ref rft) = receiver_type_inferred {
                match rft {
                    Type::Custom(n) if n == "f64" => {
                        self.assignment_float_target_type = Some(Type::Custom("f64".to_string()));
                    }
                    Type::Custom(n) if n == "f32" => {
                        self.assignment_float_target_type = Some(Type::Custom("f32".to_string()));
                    }
                    Type::Float => {
                        self.assignment_float_target_type = Some(Type::Custom("f64".to_string()));
                    }
                    _ => {}
                }
            }
        }

        let args_vec: Vec<String> = arguments
            .iter()
            .enumerate()
            .map(|(i, (_label, arg))| {
                let receiver_is_map = self.infer_expression_type(object).as_ref().is_some_and(
                    crate::codegen::rust::stdlib_method_traits::is_map_type,
                ) || type_name.as_ref().is_some_and(
                    |n| crate::codegen::rust::stdlib_method_traits::is_map_type_name(n),
                ) || match object {
                    Expression::Identifier { name, .. } => self
                        .current_function_params
                        .iter()
                        .find(|p| p.name == *name)
                        .map(|p| &p.type_)
                        .or_else(|| self.local_var_types.get(name))
                        .is_some_and(crate::codegen::rust::stdlib_method_traits::is_map_type),
                    _ => false,
                };
                let receiver_is_set = self.infer_expression_type(object).as_ref().is_some_and(
                    crate::codegen::rust::stdlib_method_traits::is_set_type,
                ) || type_name.as_ref().is_some_and(
                    |n| crate::codegen::rust::stdlib_method_traits::is_set_type_name(n),
                ) || match object {
                    Expression::Identifier { name, .. } => self
                        .current_function_params
                        .iter()
                        .find(|p| p.name == *name)
                        .map(|p| &p.type_)
                        .or_else(|| self.local_var_types.get(name))
                        .is_some_and(crate::codegen::rust::stdlib_method_traits::is_set_type),
                    _ => false,
                };
                let receiver_type_name_owned = type_name.clone().or_else(|| {
                    self.mc_infer_method_receiver_type_name(object)
                }).or_else(|| {
                    if let Expression::Identifier { name, .. } = object {
                        self.current_function_params
                            .iter()
                            .find(|p| p.name == *name)
                            .and_then(|p| Self::type_to_name(&p.type_))
                    } else {
                        None
                    }
                });
                let receiver_type_name = receiver_type_name_owned.as_deref();
                let is_external_module_method = matches!(
                    object,
                    Expression::Identifier { name, .. }
                        if name.chars().next().is_some_and(|c| c.is_lowercase())
                );
                let external_module_mut_reborrow = is_external_module_method
                    && i == 0
                    && matches!(
                        arg,
                        Expression::Identifier { name, .. }
                            if self.inferred_mut_borrowed_params.contains(name)
                    );

                let call_site_sig = self.mc_select_call_site_signature(
                    object,
                    method,
                    arguments,
                    method_signature,
                );

                let sig_for_effective = call_site_sig.as_ref().or(method_signature.as_ref());
                let is_map_key_arg = i == 0
                    && receiver_is_map
                    && crate::codegen::rust::stdlib_method_traits::method_is_map_key_qualified(
                        method,
                        receiver_type_name,
                        &self.signature_registry,
                    );
                let is_set_key_arg = i == 0
                    && receiver_is_set
                    && crate::codegen::rust::stdlib_method_traits::method_arg_needs_auto_borrow_at_index(
                        method,
                        receiver_type_name,
                        &self.signature_registry,
                        i,
                    );
                let is_collection_key_arg = sig_for_effective
                    .is_some_and(|sig| {
                        crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
                            sig,
                            i,
                            receiver_type_name,
                        )
                    })
                    || is_map_key_arg
                    || is_set_key_arg;
                let effective_ownership = if external_module_mut_reborrow {
                    Some(OwnershipMode::MutBorrowed)
                } else {
                    sig_for_effective.map(|sig| {
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                            sig, i, receiver_type_name,
                        )
                    })
                };

                // TDD FIX: Suppress auto-clone for FieldAccess when method expects Borrowed
                // Bug: ingredient.item_id generates .clone(), then & is added -> &cloned_value
                // Fix: Suppress clone when param expects Borrowed -> just add & to field
                let param_expects_borrowed = effective_ownership
                    .is_some_and(|o| matches!(o, OwnershipMode::Borrowed))
                    || sig_for_effective.is_some_and(|sig| {
                        let idx = sig.arg_param_index(i);
                        sig.param_types.get(idx).is_some_and(|t| {
                            matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                || crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        })
                    });

                let is_auto_borrow_target =
                    crate::codegen::rust::stdlib_method_traits::method_arg_needs_auto_borrow_at_index(
                        method,
                        receiver_type_name,
                        &self.signature_registry,
                        i,
                    );

                let prev_suppress = self.suppress_borrowed_clone;
                if (param_expects_borrowed || is_auto_borrow_target)
                    && matches!(arg, Expression::FieldAccess { .. } | Expression::Identifier { .. })
                {
                    self.suppress_borrowed_clone = true;
                }

                // CRITICAL: Reset in_field_access_object for method argument generation.
                // Same rationale as function call arguments — method arguments are
                // independent expressions, not part of a field/method/index chain.
                // TDD FIX: STRIP explicit &ref when parameter expects owned value.
                // WINDJAMMER PHILOSOPHY: The developer shouldn't need to think about &.
                // If the user writes `&object.transform` but the method takes `Transform` (owned),
                // the compiler strips the & and passes by value (Copy types) or moves.
                // Example: self.render_transform(&object.transform) → self.render_transform(object.transform)
                //
                // TDD FIX: ALSO strip explicit & for HashMap/BTreeMap key methods with &String arguments.
                // HashMap<String, V>.contains_key() expects &str, not &&String.
                // User writes: map.contains_key(&key) where key is inferred as &String
                // Compiler generates: map.contains_key(key) which auto-derefs &String to &str ✅
                let arg_to_generate = if let Expression::Unary {
                    op: crate::parser::UnaryOp::Ref,
                    operand,
                    ..
                } = arg
                {
                    let is_hashmap_key_method =
                        crate::codegen::rust::stdlib_method_traits::is_map_key_method(method) && i == 0;

                    if is_hashmap_key_method {
                        if let Expression::Identifier { .. } = &**operand {
                            operand
                        } else {
                            arg
                        }
                    } else if let Some(ref sig) = method_signature {
                        let sig_param_idx = sig.arg_param_index(i);
                        let param_is_owned = sig
                            .param_ownership
                            .get(sig_param_idx)
                            .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Owned));
                        if param_is_owned {
                            operand // Strip & — generate the inner expression
                        } else {
                            arg // Keep the & — parameter expects a reference
                        }
                    } else {
                        arg // No signature info — keep as-is
                    }
                } else {
                    arg // Not a & expression — keep as-is
                };

                // TDD FIX for E0277: Methods like retain/filter/any/all pass &T to
                // their closure, so closure params are references. Mark them as
                // borrowed so binary comparisons (id != val) generate *id != val.
                let closure_borrowed_params: Vec<String> =
                    if is_ref_closure_method(method) {
                        if let Expression::Closure { parameters, .. } = arg_to_generate {
                            let mut added = Vec::new();
                            for p in parameters.iter() {
                                if !self.borrowed_iterator_vars.contains(p) {
                                    self.borrowed_iterator_vars.insert(p.clone());
                                    added.push(p.clone());
                                }
                            }
                            added
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };

                let prev_arg_float_target = self.assignment_float_target_type.clone();
                if let Some(sig) = method_signature.as_ref() {
                    let pidx = sig.arg_param_index(i);
                    let param_ty = sig
                        .param_type_for_arg(i)
                        .or_else(|| sig.formal_param_type(pidx))
                        .or_else(|| sig.param_types.get(pidx));
                    if param_ty.is_some_and(
                        crate::codegen::rust::type_classification_utilities::is_float_type,
                    ) {
                        self.assignment_float_target_type = param_ty.cloned();
                    }
                }

                let scope = self.arg_gen_scope();
                let mut arg_str = self.generate_expression(arg_to_generate);
                self.restore_arg_gen_scope(scope);
                self.assignment_float_target_type = prev_arg_float_target;
                arg_str = self
                    .peel_copy_ref_match_binding_for_value(arg_to_generate, &arg_str);
                if let Some(name) =
                    crate::codegen::rust::call_site_borrow::borrow_target_identifier_name(arg)
                {
                    if self.emitted_rust_ref_formals.contains(&name) {
                        crate::codegen::rust::call_site_borrow::strip_redundant_borrow_on_ref_binding(
                            arg, &mut arg_str,
                        );
                    }
                }

                for p in &closure_borrowed_params {
                    self.borrowed_iterator_vars.remove(p);
                }

                // AUTO-WRAP function pointers in iterator adapter methods (before IR call-site path).
                if i == 0
                    && crate::codegen::rust::stdlib_method_traits::is_closure_taking_method(method)
                {
                    if let Expression::Identifier { name, .. } = arg_to_generate {
                        let is_fn_ptr_param = self.current_function_params.iter().any(|p| {
                            p.name == *name && matches!(p.type_, Type::FunctionPointer { .. })
                        });
                        if is_fn_ptr_param {
                            arg_str = format!("|__e| {}(__e)", arg_str);
                        }
                    }
                }

                if self.ir_cutover.call_sites {
                    let receiver_for_ir = receiver_type_name
                        .map(str::to_string)
                        .or_else(|| self.mc_infer_method_receiver_type_name(object))
                        .or_else(|| {
                            sig_for_effective.as_ref().and_then(|sig| {
                                crate::codegen::rust::stdlib_method_traits::receiver_type_from_qualified_sig(sig)
                                    .map(str::to_string)
                            })
                        })
                        .or_else(|| {
                            let hits: Vec<String> = self
                                .stdlib_method_signatures
                                .iter()
                                .filter(|(_, methods)| methods.contains_key(method))
                                .map(|(ty, _)| ty.clone())
                                .collect();
                            if hits.len() == 1 {
                                Some(hits[0].clone())
                            } else {
                                None
                            }
                        });
                    let qualified_callee =
                        crate::codegen::rust::stdlib_method_traits::module_qualified_method_name(
                            receiver_for_ir.as_deref(),
                            object,
                            method,
                            |name| self.is_imported_runtime_std_module(name),
                        );
                    if let Some(mut coerced) = self.apply_ir_call_site_coercion(
                        &self.signature_registry,
                        &qualified_callee,
                        i,
                        arg_to_generate,
                        &arg_str,
                        sig_for_effective,
                        false,
                        receiver_for_ir.as_deref(),
                        Some(arguments.len()),
                    ) {
                        if is_collection_key_arg {
                            if let Some(name) =
                                crate::codegen::rust::call_site_borrow::borrow_target_identifier_name(
                                    arg,
                                )
                                .or_else(|| {
                                    crate::codegen::rust::call_site_borrow::borrow_target_identifier_name(
                                        arg_to_generate,
                                    )
                                })
                            {
                                if self.emitted_rust_ref_formals.contains(&name)
                                    || self.identifier_already_ref(&name)
                                {
                                    crate::codegen::rust::call_site_borrow::strip_redundant_borrow_on_ref_binding(
                                        arg,
                                        &mut coerced,
                                    );
                                } else if self.current_function_params.iter().any(|p| p.name == name)
                                {
                                    crate::codegen::rust::call_site_borrow::strip_redundant_borrow_on_ref_binding(
                                        arg,
                                        &mut coerced,
                                    );
                                }
                            }
                            let binding_already_ref = crate::codegen::rust::call_site_borrow::borrow_target_identifier_name(
                                arg_to_generate,
                            )
                            .or_else(|| {
                                crate::codegen::rust::call_site_borrow::borrow_target_identifier_name(
                                    arg,
                                )
                            })
                            .is_some_and(|name| {
                                self.emitted_rust_ref_formals.contains(&name)
                                    || self.identifier_already_ref(&name)
                            });
                            if !coerced.starts_with('&')
                                && !binding_already_ref
                                && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(
                                    arg_to_generate,
                                )
                                && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(
                                    arg_to_generate,
                                )
                            {
                                coerced = format!("&{coerced}");
                            }
                        }
                        let fallback_sig = sig_for_effective
                            .cloned()
                            .or_else(|| method_signature.clone())
                            .unwrap_or_default();
                        coerced = crate::codegen::rust::call_site_borrow::maybe_borrow_owned_vec_local_for_ref_formal(
                            self,
                            &fallback_sig,
                            i,
                            arg_to_generate,
                            coerced,
                            type_name.as_deref().or(receiver_for_ir.as_deref()),
                            Some(method),
                            Some(arguments.len()),
                        );
                        let fallback_pidx = fallback_sig.arg_param_index(i);
                        let receiver_rt = receiver_for_ir
                            .as_deref()
                            .or(self.current_struct_name.as_deref())
                            .or(receiver_type_name);
                        let wants_ref = crate::ir::signature_bridge::call_site_expects_shared_borrow(
                            &fallback_sig,
                            fallback_pidx,
                        );
                        let wants_owned = (receiver_rt.is_some_and(|rt| {
                            self.resolve_method_function_signature(
                                rt,
                                method,
                                arguments.len(),
                            )
                            .is_some_and(|sig| {
                                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    &sig,
                                    sig.arg_param_index(i),
                                )
                            })
                        }) || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            &fallback_sig,
                            fallback_pidx,
                        )) && !wants_ref;
                        if let Expression::Identifier { name, .. } = arg_to_generate {
                            let receiver_is_self =
                                crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(
                                    object,
                                );
                            let caller_owned_param = self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && !self.emitted_rust_ref_formals.contains(name)
                            });
                            let is_mixed_forwarder =
                                self.current_fn_mixed_forwarder_params.contains(name);
                            if receiver_is_self && (caller_owned_param || is_mixed_forwarder) {
                                let body: Vec<_> =
                                    self.current_function_body.iter().copied().collect();
                                let if_facade_param = self
                                    .param_used_in_if_with_condition_and_branches(&body, name);
                                let mixed_forward_ref = (is_mixed_forwarder || if_facade_param)
                                    && self.in_if_condition;
                                let actual = self.infer_call_arg_actual_safety_type(
                                    arg_to_generate,
                                    coerced.as_str(),
                                );
                                let expected =
                                    crate::ir::signature_bridge::safety_type_from_signature_param(
                                        &fallback_sig,
                                        fallback_pidx,
                                    );
                                let kind =
                                    crate::ir::coercion::compute_coercion(&actual, &expected);
                                if mixed_forward_ref
                                    || (matches!(
                                        kind,
                                        crate::ir::coercion::CoercionKind::Borrow
                                            | crate::ir::coercion::CoercionKind::MutBorrow
                                    ) && !self.caller_owned_non_copy_formal(name))
                                {
                                    if coerced.ends_with(".clone()") {
                                        let base = coerced
                                            .trim_end_matches(".clone()")
                                            .trim();
                                        coerced = format!("&{base}");
                                    } else if !coerced.starts_with('&') {
                                        coerced = format!("&{coerced}");
                                    }
                                } else if matches!(kind, crate::ir::coercion::CoercionKind::Identity)
                                    && coerced.starts_with('&')
                                    && !coerced.starts_with("&mut ")
                                    && self.caller_owned_non_copy_formal(name)
                                {
                                    coerced = coerced.trim_start_matches('&').to_string();
                                } else if wants_owned
                                    && !mixed_forward_ref
                                    && !self.current_fn_forward_ref_if_params.contains(name)
                                    && coerced.starts_with('&')
                                    && !coerced.starts_with("&mut ")
                                {
                                    let base = coerced.trim_start_matches('&');
                                    coerced = if base.ends_with(".clone()") {
                                        base.to_string()
                                    } else {
                                        format!("{base}.clone()")
                                    };
                                }
                            }
                            self.apply_forward_ref_and_mixed_forwarder_call_coercion(
                                &mut coerced,
                                arg_to_generate,
                                Some(object),
                                wants_ref,
                                wants_owned,
                            );
                        }
                        self.finalize_owned_outer_formal_call_arg(
                            &mut coerced,
                            arg_to_generate,
                            wants_ref,
                            wants_owned,
                        );
                        if wants_owned
                            && !wants_ref
                            && !coerced.ends_with(".clone()")
                            && !coerced.starts_with('&')
                        {
                            if let Expression::Identifier { name, .. } = arg_to_generate {
                                // Only clone when the owned formal is reused after this call.
                                // Unconditional clone breaks consuming APIs (`Vec::push(item)`).
                                let needs_reuse_clone = self.auto_clone_analysis.as_ref().is_some_and(
                                    |a| {
                                        a.needs_clone(name, self.current_statement_idx).is_some()
                                            || a.needs_clone_anywhere(name)
                                    },
                                );
                                if needs_reuse_clone
                                    && self.caller_owned_non_copy_formal(name)
                                    && !crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(
                                        object,
                                    )
                                {
                                    coerced = format!("{coerced}.clone()");
                                }
                            }
                        }
                        if !coerced.starts_with('&') {
                            if let Some(rt) = receiver_rt.as_deref() {
                                if let Some(mut reg_sig) =
                                    self.resolve_method_function_signature(
                                        rt,
                                        method,
                                        arguments.len(),
                                    )
                                {
                                    let qualified = format!("{rt}::{method}");
                                    let refresh_keys = vec![qualified.clone()];
                                    crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                                        &mut reg_sig,
                                        &self.signature_registry,
                                        &refresh_keys,
                                    );
                                    if let Some(global) = self.global_signature_registry.as_ref() {
                                        crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                                            &mut reg_sig,
                                            global,
                                            &refresh_keys,
                                        );
                                    }
                                    let pidx = reg_sig.arg_param_index(i);
                                    if crate::ir::signature_bridge::call_site_expects_shared_borrow(
                                        &reg_sig, pidx,
                                    ) && !crate::ir::signature_bridge::call_site_expects_owned_pass(
                                        &reg_sig, pidx,
                                    ) && !matches!(arg_to_generate, Expression::Identifier { name, .. }
                                        if self.caller_owned_non_copy_formal(name))
                                        // Literals are already `&str` — do not emit `&"…"`.
                                        && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(
                                            arg_to_generate,
                                        )
                                    {
                                        crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                            &mut coerced,
                                        );
                                    }
                                }
                            }
                        }
                        self.maybe_pure_forwarding_strip_call_arg(
                            &mut coerced,
                            arg_to_generate,
                            receiver_rt.as_deref(),
                            Some(method),
                            Some(i),
                            Some(arguments.len()),
                        );
                        let mut contract_sig = receiver_rt
                            .as_deref()
                            .and_then(|rt| {
                                self.resolve_method_function_signature(
                                    rt,
                                    method,
                                    arguments.len(),
                                )
                            })
                            .unwrap_or_else(|| fallback_sig.clone());
                        if let Some(rt) = receiver_rt.as_deref() {
                            let qualified = format!("{rt}::{method}");
                            let refresh_keys = vec![qualified.clone()];
                            crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                                &mut contract_sig,
                                &self.signature_registry,
                                &refresh_keys,
                            );
                            if let Some(global) = self.global_signature_registry.as_ref() {
                                crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                                    &mut contract_sig,
                                    global,
                                    &refresh_keys,
                                );
                            }
                        }
                        let pidx = contract_sig.arg_param_index(i);
                        self.enforce_call_site_ownership_contract(
                            &mut coerced,
                            arg_to_generate,
                            &contract_sig,
                            pidx,
                        );
                        if (crate::ir::signature_bridge::call_site_expects_owned_pass(
                                &contract_sig,
                                pidx,
                            )
                            || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                &contract_sig,
                                pidx,
                            ))
                            && coerced.starts_with('&')
                            && !coerced.starts_with("&mut ")
                        {
                            coerced = coerced.trim_start_matches('&').to_string();
                        } else if crate::ir::signature_bridge::call_site_expects_shared_borrow(
                            &contract_sig,
                            pidx,
                        ) && !coerced.starts_with('&')
                            && !coerced.starts_with("&mut ")
                            && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(
                                arg_to_generate,
                            )
                            && !matches!(
                                arg_to_generate,
                                Expression::Identifier { name, .. }
                                    if self.identifier_already_ref(name)
                            )
                            && matches!(
                                arg_to_generate,
                                Expression::Identifier { .. } | Expression::FieldAccess { .. }
                            )
                        {
                            coerced = format!("&{coerced}");
                        }
                        return coerced;
                    }
                }

                let callee_wants_str_borrow = call_site_sig.as_ref().is_some_and(|sig| {
                    let idx = sig.arg_param_index(i);
                    matches!(
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                            sig, i, receiver_type_name,
                        ),
                        OwnershipMode::Borrowed,
                    ) || sig.param_types.get(idx).is_some_and(
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref,
                    )
                });
                let callee_wants_ref_param = call_site_sig.as_ref().or(method_signature.as_ref()).is_some_and(|sig| {
                    let idx = sig.arg_param_index(i);
                    sig.param_types.get(idx).is_some_and(|t| {
                        matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    }) || matches!(
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                            sig, i, receiver_type_name,
                        ),
                        OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                    )
                });

                // Owned params need `.clone()` when the arg is a non-Copy binding
                let arg_is_inferred_borrowed_param = matches!(
                    arg_to_generate,
                    Expression::Identifier { name, .. }
                        if self.inferred_borrowed_params.contains(name)
                            || self.inferred_mut_borrowed_params.contains(name)
                );
                let is_borrowed_iter_collecting_refs = matches!(
                    arg_to_generate,
                    Expression::Identifier { name, .. }
                        if self.borrowed_iterator_vars.contains(name)
                ) && matches!(
                    &self.current_function_return_type,
                    Some(Type::Vec(inner)) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_))
                );
                if !external_module_mut_reborrow
                    && !is_collection_key_arg
                    && !is_borrowed_iter_collecting_refs
                    && !matches!(
                        arg_to_generate,
                        Expression::Identifier { name, .. }
                            if self.borrowed_iterator_vars.contains(name)
                    )
                    && !self.in_user_written_closure
                    && !matches!(arg_to_generate, Expression::Closure { .. })
                    && !callee_wants_str_borrow
                    && !callee_wants_ref_param
                    && effective_ownership.is_some_and(|o| matches!(o, OwnershipMode::Owned))
                    && (!arg_is_inferred_borrowed_param
                        || matches!(arg_to_generate, Expression::Identifier { name, .. }
                            if self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                            })))
                {
                    let inferred_ty = self.infer_expression_type(arg_to_generate);
                    let local_ty = match arg_to_generate {
                        Expression::Identifier { name, .. } => {
                            self.local_var_types.get(name.as_str())
                        }
                        _ => None,
                    };
                    let is_copy = inferred_ty
                        .as_ref()
                        .is_some_and(|t| self.is_type_copy(t))
                        || local_ty.is_some_and(|t| self.is_type_copy(t));
                    let is_ref_to_copy = !is_copy
                        && (inferred_ty.as_ref().is_some_and(|t| {
                            matches!(t, Type::Reference(inner) | Type::MutableReference(inner)
                                if self.is_type_copy(inner))
                        }) || local_ty.is_some_and(|t| {
                            matches!(t, Type::Reference(inner) | Type::MutableReference(inner)
                                if self.is_type_copy(inner))
                        }));
                    if is_ref_to_copy
                        && !arg_str.starts_with('*')
                        && !arg_str.ends_with(".clone()")
                    {
                        arg_str = format!("*{arg_str}");
                    } else if !is_copy
                        && !is_ref_to_copy
                        && !arg_str.ends_with(".clone()")
                        && !arg_str.ends_with(".to_string()")
                        && !Self::is_enum_variant_or_constructor(arg_to_generate)
                        && matches!(
                            arg_to_generate,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        )
                    {
                        let is_text = self
                            .infer_expression_type(arg_to_generate)
                            .as_ref()
                            .is_some_and(|t| {
                                crate::codegen::rust::types::is_windjammer_text_type(t)
                            });
                        if is_text {
                            let already_owned_string = if let Expression::Identifier { name, .. } =
                                arg_to_generate
                            {
                                self.current_function_params.iter().any(|p| {
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
                            } || self.infer_expression_type(arg_to_generate).as_ref().is_some_and(
                                |t| matches!(t, Type::String),
                            );
                            let static_text_borrow = sig_for_effective.is_some_and(|sig| {
                                matches!(
                                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                        sig, i, receiver_type_name,
                                    ),
                                    OwnershipMode::Borrowed,
                                )
                            });
                            if static_text_borrow {
                                if !arg_str.starts_with('&')
                                    && !arg_str.ends_with(".to_string()")
                                {
                                    let mut borrowed = arg_str.clone();
                                    crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                        &mut borrowed,
                                    );
                                    arg_str = borrowed;
                                }
                            } else if !already_owned_string {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        } else {
                            arg_str = format!("{}.clone()", arg_str);
                        }
                    }
                }

                // TDD FIX: PHASE 2 CALL-SITE OPTIMIZATION
                // Strip unnecessary .to_string() when parameter was optimized to &str
                // Example: User writes `loader.load("name".to_string())` but Phase 2 optimized
                // the signature from `fn load(self, name: String)` to `fn load(self, name: &str)`.
                // Result: Call site should be `loader.load("name")` not `loader.load("name".to_string())`
                //
                // IMPORTANT: Only strip for &str parameters, NOT &String parameters!
                // &String parameters still need .to_string() (creates String, then borrows it)
                let mut to_string_stripped_for_str_param = false;
                if let Some(ref sig) = method_signature {
                    let sig_param_idx = sig.arg_param_index(i);
                    if let Some(param_type) = sig.param_types.get(sig_param_idx) {
                        let param_is_str_slice_ref = if let Type::Reference(inner) = param_type {
                            matches!(&**inner, Type::Custom(name) if name == "str")
                        } else {
                            false
                        };
                        let callee_wants_owned_string =
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ) == OwnershipMode::Owned
                                && sig.formal_param_type(sig_param_idx).is_some_and(|t| {
                                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                        && crate::codegen::rust::types::is_windjammer_text_type(t)
                                });
                        if param_is_str_slice_ref && !callee_wants_owned_string {
                            // Preserve .to_string() only when it's a genuine type conversion
                            // (receiver is non-string, e.g. i32.to_string()). Strip it when
                            // the receiver is already a string literal ("foo".to_string()).
                            let is_type_conversion_to_string = matches!(
                                arg_to_generate,
                                Expression::MethodCall { object, method: m, .. }
                                    if m == "to_string" && !matches!(&**object,
                                        Expression::Literal { value: crate::parser::Literal::String(_), .. }
                                    )
                            );
                            if !is_type_conversion_to_string && arg_str.ends_with(".to_string()") {
                                arg_str = arg_str[..arg_str.len() - 12].to_string();
                                to_string_stripped_for_str_param = true;
                            } else if arg_str.ends_with(".into()") {
                                arg_str = arg_str[..arg_str.len() - 7].to_string();
                                to_string_stripped_for_str_param = true;
                            }
                            // Strip .into() from nested expressions (match arms, if-else blocks)
                            // where string literals were coerced to String but callee wants &str.
                            if arg_str.contains(".into()") {
                                arg_str = arg_str.replace(".into()", "");
                                to_string_stripped_for_str_param = true;
                            }
                        }
                    }
                }

                // TDD FIX: Vec index methods require usize arguments.
                // Int inference may resolve the literal to i32/u32/i64/u64 due to
                // conflicting constraints. Fix at codegen level: rewrite any
                // integer suffix to _usize for the first argument of known
                // index-taking methods.
                if i == 0
                    && crate::codegen::rust::stdlib_method_traits::is_index_taking_method(method)
                {
                    let is_int_literal = matches!(
                        arg,
                        Expression::Literal {
                            value: Literal::Int(_) | Literal::IntSuffixed(_, _),
                            ..
                        }
                    );
                    if is_int_literal {
                        let int_suffixes =
                            ["_i32", "_i64", "_u32", "_u64", "_i16", "_u16", "_i8", "_u8"];
                        for suffix in &int_suffixes {
                            if arg_str.ends_with(suffix) {
                                arg_str = format!(
                                    "{}_usize",
                                    &arg_str[..arg_str.len() - suffix.len()]
                                );
                                break;
                            }
                        }
                    }
                }

                // TDD FIX: AUTO-WRAP function pointers in iterator adapter methods.
                // Rust's .filter()/.any()/.find() on iter() yield &&T, expecting FnMut(&&T) -> bool,
                // but bare function pointers fn(&T) -> bool don't auto-deref.
                // THE WINDJAMMER WAY: Users write the natural `filter(predicate)` and the
                // compiler generates `filter(|__e| predicate(__e))`.
                if i == 0
                    && crate::codegen::rust::stdlib_method_traits::is_closure_taking_method(method)
                {
                    if let Expression::Identifier { name, .. } = arg {
                        // Wrap function pointer parameters: iter adapters expect FnMut(&&T),
                        // but fn(&T) -> bool does not auto-deref (E0631).
                        let is_fn_ptr_param = self.current_function_params.iter().any(|p| {
                            p.name == *name && matches!(p.type_, Type::FunctionPointer { .. })
                        });
                        if is_fn_ptr_param {
                            arg_str = format!("|__e| {}(__e)", arg_str);
                        }
                    }
                }

                // CALLBACK BRIDGE: When a bare function identifier is passed as a
                // callback argument and the function's parameters have been auto-borrowed,
                // wrap it in a closure so the caller's owned args are correctly borrowed.
                // e.g. server.serve(handle_request) → server.serve(|__cb0| handle_request(&__cb0))
                //
                // Skip local variables that shadow function names — they are values,
                // not callables (e.g. `Some(parent) => path.push(parent)` where
                // `parent` shadows a top-level function).
                if let Expression::Identifier { name, .. } = arg {
                    let is_local_variable = self.local_var_types.contains_key(name.as_str())
                        || self.match_arm_bindings.contains(name.as_str())
                        || self.inferred_borrowed_params.contains(name.as_str())
                        || self.inferred_mut_borrowed_params.contains(name.as_str())
                        || self.current_function_params.iter().any(|p| p.name == *name);
                    if !is_local_variable {
                    if let Some(func_sig) = self.signature_registry.get_signature(name) {
                        if !func_sig.has_self_receiver && !func_sig.is_extern {
                            let has_borrowed: Vec<usize> = func_sig
                                .param_ownership
                                .iter()
                                .enumerate()
                                .filter(|(_, o)| {
                                    matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                                })
                                .map(|(idx, _)| idx)
                                .collect();
                            if !has_borrowed.is_empty() {
                                let n = func_sig.param_ownership.len();
                                let wrapper: Vec<String> =
                                    (0..n).map(|j| format!("__cb{}", j)).collect();
                                let call: Vec<String> = (0..n)
                                    .map(|j| match func_sig.param_ownership[j] {
                                        OwnershipMode::MutBorrowed => format!("&mut __cb{}", j),
                                        OwnershipMode::Borrowed => format!("&__cb{}", j),
                                        _ => format!("__cb{}", j),
                                    })
                                    .collect();
                                arg_str = format!(
                                    "|{}| {}({})",
                                    wrapper.join(", "),
                                    name,
                                    call.join(", ")
                                );
                            }
                        }
                    }
                    }
                }

                // TDD FIX: String literal ownership conversion
                // Windjammer philosophy: "sword" should work whether parameter wants String or &String
                // CRITICAL: Do NOT convert for explicit &str parameters! Only for inferred &String.
                let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                let _param_ownership = method_signature
                    .as_ref()
                    .and_then(|sig| sig.param_ownership_for_arg(i));
                let string_literal_converted = if is_string_literal {
                    let effective_sig = type_name
                        .as_ref()
                        .and_then(|tn| {
                            self.lookup_method_signature_on_receiver_type(
                                tn,
                                method,
                                arguments.len(),
                            )
                        })
                        .or_else(|| method_signature.clone());

                    // Check what the parameter wants
                    let asref_str_module =
                        crate::codegen::rust::stdlib_method_traits::receiver_uses_asref_str_runtime_module(
                            None,
                            type_name.as_deref(),
                            |name| self.is_imported_runtime_std_module(name),
                        )
                        || matches!(
                            object,
                            Expression::Identifier { name, .. }
                                if self.is_imported_runtime_std_module(name)
                                    || crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
                                        name,
                                    )
                        );

                    let param_type = effective_sig
                        .as_ref()
                        .and_then(|sig| sig.param_type_for_arg(i));
                    let is_explicit_str_ref = param_type
                        .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref);

                    let callee_param_is_rust_str = effective_sig.as_ref().is_some_and(|sig| {
                        let pi = sig.arg_param_index(i);
                        crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                            &Some(sig.clone()),
                            pi,
                        )
                    });

                    if asref_str_module {
                        false
                    } else if is_explicit_str_ref {
                        false
                    } else if callee_param_is_rust_str {
                        false
                    } else if is_map_key_arg && !receiver_is_string_collection {
                        false
                    } else {
                        let param_is_owned = effective_ownership.is_none()
                            || matches!(effective_ownership, Some(OwnershipMode::Owned));
                        let method_takes_str_separator = effective_sig
                            .as_ref()
                            .and_then(|sig| sig.param_type_for_arg(i))
                            .is_some_and(|t| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                            })
                            || crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
                                method,
                                type_name.as_deref(),
                                &self.signature_registry,
                                i,
                            );
                        let needs_owned = (receiver_is_string_collection && param_is_owned && !method_takes_str_separator) || crate::codegen::rust::string_utilities::string_literal_needs_owned_coercion_with_enum(
                            effective_sig.as_ref(),
                            i,
                            Some(method),
                            type_name.as_deref(),
                            Some(&self.enum_variant_types),
                            None,
                        );
                        if needs_owned {
                            arg_str = format!("{}.to_string()", arg_str);
                            true
                        } else {
                            false
                        }
                    }
                } else {
                    false
                };


                if is_string_literal
                    && !string_literal_converted
                    && !crate::codegen::rust::stdlib_method_traits::receiver_uses_asref_str_runtime_module(
                        None,
                        type_name.as_deref(),
                        |name| self.is_imported_runtime_std_module(name),
                    )
                    && !matches!(
                        object,
                        Expression::Identifier { name, .. }
                            if self.is_imported_runtime_std_module(name)
                                || crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
                                    name,
                                )
                    )
                    && !crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
                        method,
                        type_name.as_deref(),
                        &self.signature_registry,
                        i,
                    )
                    && method_signature.as_ref().is_some_and(|sig| {
                        let pi = sig.arg_param_index(i);
                        !crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(sig, i)
                            && !crate::codegen::rust::call_signature_resolution::static_impl_text_borrows_at_call_site(
                            sig, pi,
                        ) && matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ),
                            OwnershipMode::Owned,
                        ) && sig.param_type_for_arg(i).is_some_and(
                            crate::codegen::rust::string_utilities::param_is_owned_string_type,
                        )
                    })
                {
                    arg_str = format!("{}.to_string()", arg_str);
                }

                // Runtime std modules (`strings.len(self.text)`) take `AsRef<str>` — borrow
                // owned string fields/vars instead of moving out of `&mut self`.
                if !is_string_literal {
                    let asref_str_module =
                        crate::codegen::rust::stdlib_method_traits::receiver_uses_asref_str_runtime_module(
                            None,
                            type_name.as_deref(),
                            |name| self.is_imported_runtime_std_module(name),
                        );
                    let param_is_string = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_type_for_arg(i))
                        .is_some_and(
                            crate::codegen::rust::string_utilities::param_is_owned_string_type,
                        );
                    let arg_is_string = crate::codegen::rust::string_utilities::expression_is_owned_string_for_asref_borrow(
                        arg_to_generate,
                        self.infer_expression_type(arg_to_generate).as_ref(),
                        &self.local_var_types,
                        &self.current_function_params,
                    );
                    if asref_str_module
                        && param_is_string
                        && (arg_is_string
                            || matches!(
                                arg_to_generate,
                                Expression::Identifier { .. } | Expression::FieldAccess { .. }
                            ))
                        && !arg_str.starts_with('&')
                        && !arg_str.ends_with(".clone()")
                    {
                        arg_str = format!("&{}", arg_str);
                    } else if type_name.as_deref().is_some_and(|tn| {
                        crate::codegen::rust::stdlib_method_traits::runtime_std_module_for_type(tn)
                            == Some("db")
                    }) && matches!(
                        method,
                        "query" | "execute" | "get_string" | "get_int" | "get_string_at"
                            | "get_int_at"
                    ) && i == 0
                        && matches!(
                            arg_to_generate,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        )
                        && !arg_str.starts_with('&')
                    {
                        arg_str = format!("&{}", arg_str);
                    }
                }

                // If we converted to owned String, do not re-borrow for stale Borrowed metadata.
                if string_literal_converted {
                    let effective_sig = type_name
                        .as_ref()
                        .and_then(|tn| {
                            self.lookup_method_signature_on_receiver_type(
                                tn,
                                method,
                                arguments.len(),
                            )
                        })
                        .or_else(|| method_signature.clone());
                    let still_borrowed = effective_sig.as_ref().is_some_and(|sig| {
                        let idx = sig.arg_param_index(i);
                        sig.param_types.get(idx).is_some_and(|ty| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
                        }) || crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                            &effective_sig,
                            idx,
                        )
                    });
                    if still_borrowed {
                        arg_str = format!("&{}", arg_str);
                    }
                }

                // TDD FIX: AUTO-CONVERT &str → String for method calls
                // When passing a Phase 2 optimized &str parameter to a method expecting owned String, convert it
                // This handles cases like: HashMap::insert(key, value) where key is &str but insert expects String
                if let Expression::Identifier { name, .. } = arg_to_generate {
                    let is_string_const = crate::codegen::rust::string_utilities::is_string_const_identifier(
                        name,
                        self.auto_clone_analysis.as_ref(),
                    );
                    let callee_effective_is_borrowed = sig_for_effective.is_some_and(|sig| {
                        matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ),
                            OwnershipMode::Borrowed,
                        )
                    });
                    let wants_string = !callee_effective_is_borrowed && method_signature.as_ref().and_then(|sig| {
                        sig.param_type_for_arg(i).map(|ty| {
                            crate::codegen::rust::string_utilities::param_is_owned_string_type(ty)
                        })
                    }).unwrap_or(false);
                    // Signature-driven: if the resolved signature says the parameter is Owned String,
                    // convert the string constant. For generic stdlib methods (push, insert) where
                    // param_types is empty, check if the receiver is a String collection.
                    let sig_says_owned_string = method_signature.as_ref()
                        .or(call_site_sig.as_ref())
                        .is_some_and(|sig| {
                            let pi = sig.arg_param_index(i);
                            let ownership_is_owned = matches!(sig.param_ownership.get(pi), Some(OwnershipMode::Owned));
                            let type_is_string = sig.param_type_for_arg(i)
                                .is_some_and(crate::codegen::rust::string_utilities::param_is_owned_string_type);
                            ownership_is_owned && type_is_string
                        });
                    // Stdlib generic collection fallback: when no typed signature exists but
                    // the receiver is a Vec<String>/Vec<string>, string constants need .to_string()
                    let method_takes_str_not_string = sig_for_effective
                        .or(method_signature.as_ref())
                        .or(call_site_sig.as_ref())
                        .and_then(|sig| sig.param_type_for_arg(i))
                        .is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        });
                    let needs_owned_string = wants_string
                        || (sig_says_owned_string && is_string_const)
                        || (receiver_is_string_collection && is_string_const && !method_takes_str_not_string);
                    if needs_owned_string && is_string_const && !arg_str.ends_with(".to_string()")
                    {
                        arg_str = format!("{}.to_string()", arg_str);
                    }

                    let is_str_ref_optimized =
                        self.str_ref_optimized_params.contains(name.as_str());

                    if is_str_ref_optimized {
                        let is_map_key = crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
                            && i == 0;
                        let param_idx_for_sig = method_signature.as_ref().map_or(i, |s| s.arg_param_index(i));
                        let callee_sig = call_site_sig
                            .clone()
                            .or(method_signature.clone())
                            .or_else(|| {
                                type_name.as_ref().and_then(|tn| {
                                    self.lookup_method_signature_on_receiver_type(
                                        tn,
                                        method,
                                        arguments.len(),
                                    )
                                })
                            });
                        let arg_is_owned_string_binding =
                            if let Expression::Identifier { name, .. } = arg_to_generate {
                                self.current_function_params.iter().any(|p| {
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
                        let callee_borrows = callee_sig.as_ref().is_some_and(|sig| {
                            matches!(
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                    sig, i, receiver_type_name,
                                ),
                                OwnershipMode::Borrowed,
                            )
                        }) || callee_sig.as_ref().is_some_and(|sig| {
                            sig.param_types
                                .get(sig.arg_param_index(i))
                                .is_some_and(
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref,
                                )
                        });
                        if !is_map_key
                            && !arg_is_owned_string_binding
                            && !callee_borrows
                            && !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                                &callee_sig,
                                param_idx_for_sig,
                            )
                        {
                            let expects_owned = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(
                                i,
                                method,
                                &callee_sig,
                            );

                            if expects_owned
                                && !arg_str.ends_with(".to_string()")
                                && !arg_str.ends_with(".clone()")
                            {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        }
                    }
                }

                // Storage methods (e.g. HashMap::insert key) — owned String keys from &str params/locals.
                if crate::codegen::rust::stdlib_method_traits::method_is_storage_qualified(
                    method,
                    receiver_type_name,
                    &self.signature_registry,
                ) && i == 0
                    && !arg_str.ends_with(".to_string()")
                    && !arg_str.starts_with('&')
                {
                    let param_wants_string = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_type_for_arg(i))
                        .is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_owned_string_type(t)
                        });
                    let arg_is_str_like = match arg_to_generate {
                        Expression::Identifier { name, .. } => {
                            let local_str = self.local_var_types.get(name).is_some_and(|t| {
                                if matches!(t, Type::String) {
                                    return true;
                                }
                                if let Type::Reference(inner) = t {
                                    return matches!(inner.as_ref(), Type::String)
                                        || matches!(inner.as_ref(), Type::Custom(s) if s == "str");
                                }
                                false
                            });
                            local_str || self.current_function_params.iter().any(|p| {
                                if p.name != *name {
                                    return false;
                                }
                                if p.type_ == Type::String {
                                    return true;
                                }
                                if let Type::Reference(inner) = &p.type_ {
                                    return matches!(inner.as_ref(), Type::String)
                                        || matches!(inner.as_ref(), Type::Custom(s) if s == "str");
                                }
                                false
                            })
                        }
                        _ => false,
                    };
                    if param_wants_string || arg_is_str_like {
                        arg_str = format!("{}.to_string()", arg_str);
                    }
                }

                if is_collection_key_arg {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);
                    // Iterator keys (`for k in map.keys()`) are already `&K` — never clone.
                    if matches!(
                        arg_to_generate,
                        Expression::Identifier { name, .. }
                            if self.borrowed_iterator_vars.contains(name)
                    ) {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut arg_str,
                        );
                    }
                }

                // Owned Copy match bindings (after `.copied()`) must not get `.clone()`.
                if let Expression::Identifier { name, .. } = arg_to_generate {
                    if self.match_arm_bindings.contains(name.as_str())
                        && self
                            .local_var_types
                            .get(name.as_str())
                            .is_some_and(|t| self.is_type_copy(t))
                    {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut arg_str,
                        );
                    }
                }

                // AUTO .clone(): Add .clone() when needed for borrowed values
                if !self.current_func_is_pure_forwarding_delegate {
                if let Expression::Identifier { name, .. } = arg {
                    let arg_is_inferred_borrowed_param = self.inferred_borrowed_params.contains(name)
                        || self.inferred_mut_borrowed_params.contains(name);
                    let param_is_mut_borrowed = method_signature
                        .as_ref()
                        .and_then(|sig| sig.param_ownership_for_arg(i))
                        .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed))
                        || method_signature.as_ref().and_then(|sig| {
                            sig.param_type_for_arg(i).map(|t| {
                                matches!(t, Type::MutableReference(_))
                            })
                        }).unwrap_or(false);
                    // Signature-driven map/set key lookup (not method-name lists).
                    let param_is_borrowed_map_key = is_collection_key_arg
                        && (method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership_for_arg(i))
                            .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed))
                            || self.borrowed_iterator_vars.contains(name));
                    // Borrowed iterator vars (for x in &vec / map.keys()) are already references.
                    // Cloning them produces owned values, changing the type.
                    let is_borrowed_iter_var = self.borrowed_iterator_vars.contains(name);
                    let is_borrowed_iter_collecting_refs = is_borrowed_iter_var
                            && matches!(
                                &self.current_function_return_type,
                                Some(Type::Vec(inner)) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_))
                            );
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if !arg_is_inferred_borrowed_param
                            && !external_module_mut_reborrow
                            && !param_is_mut_borrowed
                            && !param_is_borrowed_map_key
                            && !is_borrowed_iter_var
                            && !is_borrowed_iter_collecting_refs
                            && !param_expects_borrowed
                            && !is_auto_borrow_target
                            && !self.param_used_in_prior_field_extract_call(name)
                            && analysis
                                .needs_clone(name, self.current_statement_idx)
                                .is_some()
                            && !arg_str.ends_with(".clone()")
                            && !arg_str.starts_with('*')
                        {
                            let ref_to_copy = self.infer_expression_type(arg_to_generate)
                                .as_ref()
                                .is_some_and(|t| matches!(t, Type::Reference(inner) | Type::MutableReference(inner) if self.is_type_copy(inner)));
                            if ref_to_copy {
                                arg_str = format!("*{arg_str}");
                            } else {
                                arg_str = format!("{}.clone()", arg_str);
                            }
                        }
                    }
                }
                }

                let clone_sig = call_site_sig.clone().or_else(|| method_signature.clone());
                if !self.current_func_is_pure_forwarding_delegate
                    && !callee_wants_ref_param
                    && !is_borrowed_iter_collecting_refs
                    && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_clone(
                    arg,
                    &arg_str,
                    method,
                    i,
                    &clone_sig,
                    &self.borrowed_iterator_vars,
                    &self.current_function_params,
                    &self.inferred_borrowed_params,
                    &self.current_function_return_type,
                ) {
                    let inferred_ty = self.infer_expression_type(arg);
                    let is_ref_to_copy = inferred_ty
                        .as_ref()
                        .is_some_and(|t| matches!(t, Type::Reference(inner) | Type::MutableReference(inner) if self.is_type_copy(inner)));
                    let is_already_copy = inferred_ty
                        .as_ref()
                        .is_some_and(|t| self.is_type_copy(t))
                        || matches!(arg, Expression::Identifier { name, .. }
                            if self.local_var_types.get(name.as_str()).is_some_and(|t| {
                                self.is_type_copy(t)
                            }));
                    let is_borrowed_var = matches!(arg, Expression::Identifier { name, .. } if self.borrowed_iterator_vars.contains(name));
                    if is_ref_to_copy && !arg_str.starts_with('*') {
                        arg_str = format!("*{arg_str}");
                    } else if is_already_copy && is_borrowed_var && !arg_str.starts_with('*') {
                        arg_str = format!("*{arg_str}");
                    } else if is_already_copy {
                        // Owned Copy (e.g. after Option<&T>.copied()) — pass by value, no .clone().
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut arg_str,
                        );
                    } else if !is_already_copy {
                        arg_str = format!("{}.clone()", arg_str);
                    }
                }

                // DOGFOODING FIX: Vec indexing vec[idx] passed to owned param (e.g. push)
                // should_add_clone handles Identifier/FieldAccess; Index needs explicit check
                // Vec::push uses stdlib heuristics (method_signature=None) - param 0 expects Owned
                if let Expression::Index { .. } = arg {
                    let param_expects_owned = method_signature
                        .as_ref()
                        .or(call_site_sig.as_ref())
                        .and_then(|sig| sig.param_ownership_for_arg(i))
                        .is_some_and(|&o| matches!(o, OwnershipMode::Owned));
                    if param_expects_owned && !arg_str.ends_with(".clone()") {
                        let inferred = self.infer_expression_type(arg);
                        let is_copy = inferred.as_ref().is_some_and(|t| self.is_type_copy(t));
                        let is_value_constructor = Self::is_enum_variant_or_constructor(arg);
                        if is_copy || is_value_constructor {
                            if arg_str.starts_with("&") {
                                arg_str = arg_str
                                    .strip_prefix('&')
                                    .unwrap_or(&arg_str)
                                    .to_string();
                            }
                        } else {
                            if arg_str.starts_with("&") {
                                arg_str = format!("({}).clone()", arg_str);
                            } else {
                                arg_str = format!("{}.clone()", arg_str);
                            }
                        }
                    }
                }

                let arg_already_rust_ref = matches!(
                    arg_to_generate,
                    Expression::Identifier { name, .. }
                        if self.identifier_already_ref(name)
                            || self.str_ref_optimized_params.contains(name.as_str())
                            || self.inferred_borrowed_params.contains(name)
                );

                // Phase 3: unified call-site borrow lowering (param_expects_borrowed path)
                let call_site_sig = self.mc_select_call_site_signature(
                    object,
                    method,
                    arguments,
                    method_signature,
                );

                // Cross-crate string literal → String conversion:
                // If the string literal was not already converted but the call-site signature
                // expects an owned String parameter, add .to_string() now.
                if is_string_literal && !string_literal_converted && !arg_str.ends_with(".to_string()") {
                    let runtime_std_asref = matches!(
                        object,
                        Expression::Identifier { name, .. }
                            if self.is_imported_runtime_std_module(name)
                                || crate::codegen::rust::stdlib_method_traits::runtime_std_module_uses_asref_str(
                                    name,
                                )
                    );
                    let needs_conversion = !runtime_std_asref
                        && call_site_sig.as_ref().is_some_and(|sig| {
                        let pi = sig.arg_param_index(i);
                        !crate::codegen::rust::call_signature_resolution::static_impl_text_borrows_at_call_site(
                            sig, pi,
                        ) && matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ),
                            OwnershipMode::Owned,
                        ) && sig.param_type_for_arg(i).is_some_and(
                            crate::codegen::rust::string_utilities::param_is_owned_string_type,
                        )
                    });
                    if needs_conversion {
                        arg_str = format!("{}.to_string()", arg_str);
                    }
                }

                let mut borrow_decision =
                    crate::codegen::rust::call_site_borrow::CallSiteBorrowDecision::default();
                if let Some(ref sig) = call_site_sig {
                    let formal_is_copy = sig
                        .formal_param_type(sig.arg_param_index(i))
                        .is_some_and(|t| self.is_type_copy(t));
                    borrow_decision =
                        crate::codegen::rust::call_site_borrow::should_borrow_at_call_site_with_copy_check(
                            sig,
                            i,
                            arg_to_generate,
                            &arg_str,
                            method,
                            arg_already_rust_ref,
                            receiver_type_name,
                            formal_is_copy,
                        );
                } else if let Some(receiver_tn) = self
                    .mc_infer_method_receiver_type_name(object)
                    .or_else(|| self.infer_type_name(object))
                {
                    let resolved_sig = self
                        .resolve_call_signature_with_global(
                            &format!("{receiver_tn}::{method}"),
                            Some(receiver_tn.as_str()),
                            arguments.len(),
                        )
                        .map(|r| r.sig)
                        .or_else(|| {
                            self.lookup_method_signature_on_receiver_type(
                                &receiver_tn,
                                method,
                                arguments.len(),
                            )
                        });
                    if let Some(sig) = resolved_sig {
                        let formal_is_copy = sig
                            .formal_param_type(sig.arg_param_index(i))
                            .is_some_and(|t| self.is_type_copy(t));
                        borrow_decision =
                            crate::codegen::rust::call_site_borrow::should_borrow_at_call_site_with_copy_check(
                                &sig,
                                i,
                                arg_to_generate,
                                &arg_str,
                                method,
                                arg_already_rust_ref,
                                receiver_type_name,
                                formal_is_copy,
                            );
                    } else if is_collection_key_arg && !arg_str.starts_with('&')
                        && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_to_generate)
                    {
                        borrow_decision.add_ref = true;
                    }
                } else if is_collection_key_arg && !arg_str.starts_with('&')
                    && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_to_generate)
                {
                    borrow_decision.add_ref = true;
                }

                // Match-arm payloads: borrow at call sites (enum destructure binds owned values).
                if let Expression::Identifier { name, .. } = arg_to_generate {
                    if self.match_arm_bindings.contains(name.as_str()) && !arg_str.starts_with('&') {
                        borrow_decision.add_ref = true;
                        borrow_decision.strip_clone = true;
                    }
                }

                // Codegen-local guards preserved from pre-Phase-3 path
                if matches!(arg_to_generate, Expression::Identifier { name, .. }
                    if self.identifier_already_ref(name))
                {
                    borrow_decision.add_ref = false;
                }
                if matches!(arg_to_generate, Expression::Identifier { name, .. }
                    if self.str_ref_optimized_params.contains(name.as_str()))
                    && is_collection_key_arg
                {
                    borrow_decision.add_ref = false;
                }
                if matches!(arg_to_generate, Expression::StructLiteral { .. })
                    && (param_expects_borrowed || is_collection_key_arg)
                    && !arg_str.starts_with('&')
                {
                    borrow_decision.add_ref = false;
                }
                if let Some(ref sig) = call_site_sig {
                    let sig_param_idx = sig.arg_param_index(i);
                    let effective = crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                        sig, i, receiver_type_name,
                    );
                    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                        let formal_is_non_ref_copy = sig
                            .formal_param_type(sig_param_idx)
                            .is_some_and(|t| {
                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    && self.is_type_copy(t)
                            });
                        if matches!(param_ty, Type::Reference(_) | Type::MutableReference(_))
                            && effective != OwnershipMode::Owned
                            && !arg_str.starts_with('&')
                            && !formal_is_non_ref_copy
                        {
                            let param_is_str_ref = crate::codegen::rust::string_utilities::param_is_rust_str_ref(
                                param_ty,
                            );
                            let arg_already_str_ref = matches!(
                                arg_to_generate,
                                Expression::Identifier { name, .. }
                                    if self.inferred_borrowed_params.contains(name)
                                        || self.str_ref_optimized_params.contains(name.as_str())
                                        || self.current_function_params.iter().any(|p| {
                                            p.name == *name
                                                && matches!(
                                                    &p.type_,
                                                    Type::Reference(inner)
                                                        if matches!(
                                                            inner.as_ref(),
                                                            Type::Custom(s) if s == "str"
                                                        )
                                                )
                                        })
                            );
                            let arg_is_str_literal = crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_to_generate);
                            if !(param_is_str_ref && (arg_already_str_ref || arg_is_str_literal)) && !arg_already_rust_ref {
                                borrow_decision.add_ref = true;
                            }
                        }
                    }
                }

                if borrow_decision.strip_clone {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);
                    // Owned-path may have added `.to_string()` before we knew callee takes &str.
                    // But preserve when the receiver is a non-string type (genuine conversion).
                    let is_type_conversion_to_string = matches!(
                        arg_to_generate,
                        Expression::MethodCall { object, method: m, .. }
                            if m == "to_string" && !matches!(&**object,
                                Expression::Literal { value: crate::parser::Literal::String(_), .. }
                            )
                    );
                    if !is_type_conversion_to_string
                        && param_expects_borrowed && arg_str.ends_with(".to_string()")
                        && method_signature.as_ref().and_then(|sig| {
                            sig.param_type_for_arg(i)
                        }).is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        }) {
                            arg_str = arg_str[..arg_str.len() - 12].to_string();
                        }
                }

                if borrow_decision.add_ref
                    && !arg_str.starts_with('&')
                    && (matches!(arg_to_generate, Expression::Identifier { name, .. }
                        if self.match_arm_bindings.contains(name.as_str()))
                        || !matches!(effective_ownership, Some(OwnershipMode::Owned))
                        || is_collection_key_arg)
                {
                    crate::codegen::rust::rust_coercion_rules::Coercion::Borrow
                        .apply(&mut arg_str);
                }

                // `if let Some(x) = &self.opt` — pass owned values via `.clone()`, not `&x` / `&mut x`.
                // Skip when callee expects a borrow (HashMap keys, &str params, etc.).
                // Match-arm bindings (owned enum payloads) need `&binding`, not `.clone()`.
                if !param_expects_borrowed && !is_collection_key_arg && !external_module_mut_reborrow {
                    if let Expression::Identifier { name, .. } = arg_to_generate {
                    if self.match_arm_bindings.contains(name.as_str()) {
                        // Fall through to should_add_ref / borrow_decision below.
                    } else {
                    let inferred = self.infer_expression_type(arg_to_generate);
                    let is_ref_binding = inferred
                        .as_ref()
                        .is_some_and(|t| {
                            matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        })
                        || self.match_arm_bindings.contains(name.as_str());
                    // Skip Copy types — they don't need cloning (e.g. i32 from enum destructure,
                    // or &u32 from HashMap.get match arms after peel_copy_ref).
                    let is_copy = inferred.as_ref().is_some_and(|t| match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => self
                            .is_type_copy(inner.as_ref()),
                        other => crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(other),
                    });
                    if is_ref_binding && !is_copy {
                        if let Some(ref sig) = sig_for_effective {
                            let effective =
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                    sig, i, receiver_type_name,
                                );
                            let param_is_mut_borrowed =
                                matches!(effective, OwnershipMode::MutBorrowed);
                            let param_is_owned = matches!(effective, OwnershipMode::Owned);
                            if param_is_owned
                                && !param_is_mut_borrowed
                                && !arg_str.ends_with(".clone()")
                            {
                                let base = arg_str
                                    .trim_start_matches("&mut ")
                                    .trim_start_matches('&');
                                arg_str = format!("{}.clone()", base);
                            }
                        }
                    }
                    }
                    }
                }

                // AUTO-MUT-BORROW: Add &mut when parameter expects MutBorrowed
                if external_module_mut_reborrow {
                    crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                        arg,
                        &mut arg_str,
                        &self.current_function_params,
                        &self.inferred_mut_borrowed_params,
                    );
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut arg_str);
                } else if let Some(ref sig) = method_signature {
                    let sig_param_idx = sig.arg_param_index(i);
                    let ownership_is_mut = sig
                        .param_ownership
                        .get(sig_param_idx)
                        .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed));
                    let type_is_mut_ref = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                        matches!(t, Type::MutableReference(_))
                    });
                    let param_is_mut_borrowed = ownership_is_mut || type_is_mut_ref;
                    let param_wants_owned_value = sig.param_types.get(sig_param_idx).is_some_and(|ty| {
                        matches!(ty, Type::Custom(n) if n == "World" || n == "Entity")
                    });
                    if param_is_mut_borrowed && !param_wants_owned_value {
                        crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                            arg,
                            &mut arg_str,
                            &self.current_function_params,
                            &self.inferred_mut_borrowed_params,
                        );
                        // Owned-reuse clone must not survive a MutBorrowed formal.
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut arg_str,
                        );
                    }
                }

                // AUTO-REF: Add & when parameter expects reference but arg is owned
                // Skip when .to_string() was stripped for &str param — the result is
                // already a bare literal/value that is &str, adding & would create &&str.
                let callee_expects_owned = matches!(effective_ownership, Some(OwnershipMode::Owned))
                    || method_signature.as_ref().is_some_and(|sig| {
                        matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ),
                            OwnershipMode::Owned,
                        )
                    })
                    || call_site_sig.as_ref().is_some_and(|sig| {
                        matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig, i, receiver_type_name,
                            ),
                            OwnershipMode::Owned,
                        )
                    });
                // Borrowed formal → owned callee (e.g. key_in_latest_base → has_key(key: Key)).
                if callee_expects_owned {
                    if let Expression::Identifier { name, .. } = arg_to_generate {
                        let param_is_borrowed_formal = self.emitted_rust_ref_formals.contains(name.as_str())
                            || self.inferred_borrowed_params.contains(name.as_str())
                            || self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && matches!(&p.type_, Type::Reference(_))
                            });
                        let callee_wj_owned_non_copy = method_signature
                            .as_ref()
                            .or(call_site_sig.as_ref())
                            .and_then(|sig| {
                                let idx = sig.arg_param_index(i);
                                sig.formal_param_type(idx)
                            })
                            .is_some_and(|t| {
                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    && !self.is_type_copy(t)
                            });
                        if param_is_borrowed_formal
                            && callee_wj_owned_non_copy
                            && !arg_str.ends_with(".clone()")
                        {
                            let inner = arg_str.trim_start_matches('&');
                            arg_str = format!("{inner}.clone()");
                        }
                    }
                }
                if !string_literal_converted
                    && !to_string_stripped_for_str_param
                    && !callee_expects_owned
                    && call_site_sig.is_none()
                {
                    let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                        arg_to_generate,
                        &arg_str,
                        method,
                        i,
                        method_signature,
                        &self.usize_variables,
                        &self.current_function_params,
                        &self.borrowed_iterator_vars,
                        &self.inferred_borrowed_params,
                        arguments.len(),
                        type_name.as_deref(),
                        Some(&self.local_var_types),
                        Some(&self.stdlib_method_signatures),
                        Some(&self.method_signatures_by_type),
                        &self.match_arm_bindings,
                        &self.str_ref_optimized_params,
                    );
                    if should_ref {
                        let sig_owns_string_param = method_signature
                            .as_ref()
                            .or(call_site_sig.as_ref())
                            .is_some_and(|sig| {
                                let pi = sig.arg_param_index(i);
                                matches!(sig.param_ownership.get(pi), Some(OwnershipMode::Owned))
                            })
                            && matches!(arg_to_generate, Expression::Identifier { name, .. }
                            if !self.inferred_borrowed_params.contains(name.as_str())
                                && !self.borrowed_iterator_vars.contains(name)
                                && self
                                    .infer_expression_type(arg_to_generate)
                                    .is_some_and(|t| matches!(t, Type::String)));
                        if !sig_owns_string_param {
                            borrow_decision.add_ref = true;
                        }
                    }
                }

                // Map/set key methods: `key: &str` bindings must not become `&&str` (E0277).
                // Do not treat body-inferred borrow on owned `String` formals as already-ref —
                // those bindings still need `&key` at the lookup site.
                if is_collection_key_arg {
                    if let Expression::Identifier { name, .. } = arg_to_generate {
                        let key_already_str_ref = self.str_ref_optimized_params.contains(name.as_str())
                            || self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && (matches!(
                                        &p.type_,
                                        Type::Reference(inner)
                                            if matches!(inner.as_ref(), Type::Custom(s) if s == "str")
                                    ) || matches!(&p.type_, Type::Custom(s) if s == "str"))
                            });
                        if key_already_str_ref {
                            borrow_decision.add_ref = false;
                            if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                                arg_str = arg_str.trim_start_matches('&').to_string();
                            }
                        }
                    }
                }

                let callee_formal_is_copy_mc = method_signature.as_ref().is_some_and(|sig| {
                    let idx = sig.arg_param_index(i);
                    sig.formal_param_type(idx)
                        .is_some_and(|t| self.is_type_copy(t))
                });
                if borrow_decision.add_ref
                    && !arg_str.starts_with('&')
                    && !arg_already_rust_ref
                    && !callee_formal_is_copy_mc
                    && (matches!(arg_to_generate, Expression::Identifier { name, .. }
                        if self.match_arm_bindings.contains(name.as_str()))
                        || ((!callee_expects_owned || is_collection_key_arg)
                            && (!matches!(effective_ownership, Some(OwnershipMode::Owned))
                                || is_collection_key_arg)
                            && !matches!(effective_ownership, Some(OwnershipMode::Borrowed))
                            && !method_signature.as_ref().is_some_and(|sig| {
                                let idx = sig.arg_param_index(i);
                                matches!(sig.param_ownership.get(idx), Some(OwnershipMode::Owned))
                                    && !matches!(
                                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                            sig, i, receiver_type_name,
                                        ),
                                        OwnershipMode::Borrowed,
                                    )
                            })))
                {
                    if let Expression::Cast { .. } = arg_to_generate {
                        arg_str = format!("&({})", arg_str);
                    } else {
                        crate::codegen::rust::call_site_borrow::apply_call_site_borrow(
                            &crate::codegen::rust::call_site_borrow::CallSiteBorrowDecision {
                                add_ref: true,
                                ..Default::default()
                            },
                            &mut arg_str,
                        );
                    }
                }

                let sig_param_idx = method_signature
                    .as_ref()
                    .map(|sig| sig.arg_param_index(i))
                    .unwrap_or(i);
                arg_str = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                    method_signature,
                    sig_param_idx,
                    arg_to_generate,
                    arg_str,
                    string_literal_converted || to_string_stripped_for_str_param,
                );

                // Borrow owned `string` locals for `&str` formals when qualified lookup failed.
                if !is_map_key_arg
                    && !is_string_literal
                    && !arg_str.starts_with('&')
                    && !string_literal_converted
                    && !to_string_stripped_for_str_param
                    && !arg_str.ends_with(".to_string()")
                {
                    let receiver_type_name = type_name.clone().or_else(|| {
                        self.infer_expression_type(object).and_then(|t| match t {
                            Type::Custom(name) => Some(name),
                            Type::Reference(inner) | Type::MutableReference(inner) => {
                                if let Type::Custom(name) = inner.as_ref() {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                    });
                    let qualified = receiver_type_name
                        .as_deref()
                        .map(|tn| format!("{tn}::{method}"));
                    let registry_sig = receiver_type_name
                        .as_deref()
                        .and_then(|rt| {
                            self.signature_registry.find_method_on_receiver_type(
                                rt,
                                method,
                                arguments.len(),
                            )
                        })
                        .or_else(|| {
                            qualified
                                .as_ref()
                                .and_then(|q| self.signature_registry.get_signature(q))
                        })
                        .or_else(|| {
                            receiver_type_name.is_none().then(|| {
                                self.signature_registry
                                    .find_signature_by_name_and_arg_count(method, arguments.len())
                            }).flatten()
                        });
                    let sig_for_str_borrow = call_site_sig.as_ref().or(registry_sig);
                    let wants_str = sig_for_str_borrow.is_some_and(|sig| {
                        matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                sig,
                                i,
                                receiver_type_name.as_deref(),
                            ),
                            OwnershipMode::Borrowed,
                        )
                    });
                    if wants_str && !callee_expects_owned {
                        let param_is_registry_owned = sig_for_str_borrow.is_some_and(|sig| {
                            matches!(
                                sig.param_ownership.get(sig.arg_param_index(i)),
                                Some(OwnershipMode::Owned)
                            )
                        });
                        let formal_is_plain_owned_string = sig_for_str_borrow.is_some_and(|sig| {
                            let idx = sig.arg_param_index(i);
                            sig.formal_param_type(idx).is_some_and(|t| {
                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    && crate::codegen::rust::types::is_windjammer_text_type(t)
                            })
                        });
                        if param_is_registry_owned {
                            // trait/port metadata: owned string formal
                        } else if !formal_is_plain_owned_string {
                            let param_is_borrowed_text = sig_for_str_borrow.is_some_and(|sig| {
                                sig.param_type_for_arg(i).is_some_and(|t| {
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                                        || matches!(
                                            t,
                                            Type::Reference(inner)
                                                if crate::codegen::rust::types::is_windjammer_text_type(
                                                    inner,
                                                )
                                        )
                                })
                            });
                            if param_is_borrowed_text
                                && self
                                    .infer_expression_type(arg_to_generate)
                                    .as_ref()
                                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                            {
                                // String literals are already &str — adding & creates &&str.
                                // This covers both bare literals and "lit".to_string().
                                let is_string_literal_expr = matches!(
                                    arg_to_generate,
                                    Expression::Literal { value: Literal::String(_), .. }
                                ) || matches!(
                                    arg_to_generate,
                                    Expression::MethodCall { method: m, object, .. }
                                    if *m == "to_string" && matches!(
                                        &**object,
                                        Expression::Literal { value: Literal::String(_), .. }
                                    )
                                );
                                if !is_string_literal_expr {
                                    let mut borrowed = arg_str.clone();
                                    crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                        &mut borrowed,
                                    );
                                    arg_str = borrowed;
                                }
                            }
                        }
                    }
                }

                // AUTO-BORROW: Methods that take &T or &[T] should auto-borrow
                // when given owned values. Eliminates Rust leakage in .wj files.
                let is_auto_borrow =
                    crate::codegen::rust::stdlib_method_traits::method_arg_needs_auto_borrow_at_index(
                        method,
                        receiver_type_name,
                        &self.signature_registry,
                        i,
                    );
                let is_map_method = crate::codegen::rust::stdlib_method_traits::is_map_key_method(method)
                    && i == 0
                    && (self.infer_expression_type(object).as_ref().is_some_and(
                        crate::codegen::rust::stdlib_method_traits::is_map_type,
                    ) || self
                        .infer_type_name(object)
                        .as_ref()
                        .is_some_and(|n| crate::codegen::rust::stdlib_method_traits::is_map_type_name(n)));
                let is_set_method = crate::codegen::rust::stdlib_method_traits::is_set_lookup_method(method)
                    && i == 0
                    && (self.infer_expression_type(object).as_ref().is_some_and(
                        crate::codegen::rust::stdlib_method_traits::is_set_type,
                    ) || self
                        .infer_type_name(object)
                        .as_ref()
                        .is_some_and(|n| crate::codegen::rust::stdlib_method_traits::is_set_type_name(n)));
                if (is_auto_borrow || is_map_method || is_set_method)
                    && (is_auto_borrow || i == 0)
                {
                    let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                    let arg_is_windjammer_str = match arg_to_generate {
                        Expression::Identifier { name, .. } => {
                            self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && (crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                        || matches!(
                                            &p.type_,
                                            Type::Reference(inner)
                                                if crate::codegen::rust::types::is_windjammer_text_type(inner)
                                        ))
                            }) || self.inferred_borrowed_params.contains(name)
                        }
                        _ => false,
                    };
                    let arg_already_ref = match arg_to_generate {
                        Expression::Identifier { name, .. } => self.identifier_already_ref(name),
                        _ => {
                            let arg_ty = self.infer_expression_type(arg);
                            arg_ty.as_ref().is_some_and(|t| {
                                matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    || matches!(t, Type::Custom(n) if n == "&str")
                            }) || match arg_to_generate {
                                Expression::Identifier { name, .. } =>
                                    self.borrowed_iterator_vars.contains(name),
                                _ => false,
                            }
                        }
                    };
                    if !is_string_literal
                        && !arg_is_windjammer_str
                        && !arg_str.starts_with('&')
                        && !arg_already_ref
                    {
                        let needs_borrow = match arg {
                            Expression::Unary {
                                op: crate::parser::UnaryOp::Deref,
                                ..
                            } => false,
                            Expression::Identifier { .. }
                            | Expression::FieldAccess { .. }
                            | Expression::MethodCall { .. }
                            | Expression::Tuple { .. }
                            | Expression::Binary { .. }
                            | Expression::Unary { .. }
                            | Expression::Cast { .. } => true,
                            _ => false,
                        };
                        if needs_borrow {
                            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                &mut arg_str,
                            );
                        }
                    }
                }

                // AUTO-CAST int → float
                {
                    let cast_sig = type_name
                        .as_ref()
                        .and_then(|tn| {
                            self.lookup_method_signature_on_receiver_type(
                                tn,
                                method,
                                arguments.len(),
                            )
                        })
                        .or_else(|| method_signature.clone());
                    if let Some(sig) = cast_sig {
                        let sig_param_idx = sig.arg_param_index(i);
                        let qualified_key = type_name
                            .as_ref()
                            .map(|tn| format!("{}::{}", tn, method));
                        let skip_cast = self.should_skip_int_to_float_auto_cast_with_global(
                            type_name.as_deref(),
                            method,
                            qualified_key.as_deref(),
                        );
                        if !skip_cast {
                            let param_ty = sig
                                .param_type_for_arg(i)
                                .or_else(|| sig.param_types.get(sig_param_idx));
                            if let Some(param_ty) = param_ty {
                                let arg_ty = self.infer_expression_type(arg_to_generate);
                                crate::codegen::rust::type_classification_utilities::maybe_cast_int_arg_to_float(
                                    &mut arg_str,
                                    arg_to_generate,
                                    param_ty,
                                    arg_ty.as_ref(),
                                );
                            }
                        }
                    }
                }

                // Restore suppress flag
                self.suppress_borrowed_clone = prev_suppress;

                let effective_sig = type_name
                    .as_ref()
                    .and_then(|tn| {
                        self.lookup_method_signature_on_receiver_type(
                            tn,
                            method,
                            arguments.len(),
                        )
                    })
                    .or_else(|| method_signature.clone());

                if !string_literal_converted {
                    crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
                        effective_sig.as_ref(),
                        i,
                        Some(method),
                        arg_to_generate,
                        &mut arg_str,
                        type_name.as_deref(),
                        Some(&self.enum_variant_types),
                        None,
                    );
                }

                let arg_already_rust_ref_for_text = matches!(
                    arg_to_generate,
                    Expression::Identifier { name, .. }
                        if self.identifier_already_ref(name)
                            || self.str_ref_optimized_params.contains(name.as_str())
                            || self.inferred_borrowed_params.contains(name)
                );

                crate::codegen::rust::string_utilities::finalize_borrowed_text_call_site_arg(
                    call_site_sig
                        .as_ref()
                        .or(method_signature.as_ref())
                        .or(effective_sig.as_ref()),
                    i,
                    receiver_type_name,
                    arg_to_generate,
                    &mut arg_str,
                    arg_already_rust_ref_for_text,
                );

                if is_collection_key_arg {
                    let arg_binding_already_shared_ref =
                        crate::codegen::rust::call_site_borrow::borrow_target_identifier_name(arg)
                            .is_some_and(|name| {
                                self.emitted_rust_ref_formals.contains(&name)
                                    || self.identifier_already_ref(&name)
                            });
                    crate::codegen::rust::call_site_borrow::finalize_collection_key_call_site_arg(
                        call_site_sig
                            .as_ref()
                            .or(method_signature.as_ref())
                            .or(effective_sig.as_ref()),
                        i,
                        arg_to_generate,
                        &mut arg_str,
                        arg_already_rust_ref_for_text,
                        receiver_type_name,
                        arg_binding_already_shared_ref,
                    );
                }

                if let Some(sig_for_vec) = call_site_sig
                    .as_ref()
                    .or(method_signature.as_ref())
                    .or(effective_sig.as_ref())
                {
                    arg_str = crate::codegen::rust::call_site_borrow::maybe_borrow_owned_vec_local_for_ref_formal(
                        self,
                        sig_for_vec,
                        i,
                        arg_to_generate,
                        arg_str,
                        type_name.as_deref().or(receiver_type_name),
                        Some(method),
                        Some(arguments.len()),
                    );
                }

                if i == 0
                    && !arg_str.starts_with('&')
                    && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(
                        arg_to_generate,
                    )
                {
                    let sig_for_borrow = call_site_sig
                        .as_ref()
                        .or(method_signature.as_ref())
                        .or(effective_sig.as_ref());
                    if sig_for_borrow.is_some_and(|sig| {
                        crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow(
                            Some(sig),
                            i,
                        )
                    }) {
                        arg_str = format!("&{arg_str}");
                    }
                }

                // Final guard: Copy-type formals should never have `&` added.
                // Multiple paths above can add `&` based on analyzer ownership metadata,
                // but Copy types in generated Rust are always emitted by-value.
                // Collection key lookups (`HashMap::get`) are the exception — they need `&K`.
                // Only strip when the formal type is a non-reference Copy type (e.g. NodeId, f32).
                // Reference formals (&str, &Vec<T>) still need the `&` at the call site.
                if !is_collection_key_arg {
                    let final_sig = call_site_sig.as_ref().or(method_signature.as_ref());
                    if let Some(sig) = final_sig {
                        let pidx = sig.arg_param_index(i);
                        let formal_is_non_ref_copy = sig
                            .formal_param_type(pidx)
                            .is_some_and(|t| {
                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    && self.is_type_copy(t)
                            });
                        if formal_is_non_ref_copy
                            && (arg_str.starts_with('&') && !arg_str.starts_with("&mut "))
                        {
                            arg_str = arg_str[1..].to_string();
                        }
                    }
                }

                if !self.ir_cutover.call_sites
                    && crate::codegen::rust::typed_lowering::is_typed_lowering_enabled()
                {
                    if let Some(ref sig) = sig_for_effective.cloned() {
                        let pidx = sig.arg_param_index(i);
                        let is_formal_copy = sig
                            .formal_param_type(pidx)
                            .is_some_and(|t| {
                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    && self.is_type_copy(t)
                            });
                        crate::codegen::rust::typed_lowering::correct_legacy_output(
                            sig,
                            i,
                            arg_to_generate,
                            &mut arg_str,
                            is_formal_copy,
                            is_collection_key_arg,
                        );
                    }
                }

                if !arg_str.starts_with('&') {
                    let registry_sig = type_name
                        .as_deref()
                        .or(receiver_type_name)
                        .and_then(|rt| {
                            self.resolve_method_function_signature(rt, method, arguments.len())
                        });
                        if let Some(sig) = registry_sig
                        .as_ref()
                        .or(call_site_sig.as_ref())
                        .or(method_signature.as_ref())
                    {
                        let pidx = sig.arg_param_index(i);
                        if crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx)
                            && !crate::ir::signature_bridge::call_site_expects_owned_pass(sig, pidx)
                            && !matches!(arg_to_generate, Expression::Identifier { name, .. }
                                if self.caller_owned_non_copy_formal(name))
                            // String literals are already `&str` — `&"lit"` is `&&str`.
                            && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(
                                arg_to_generate,
                            )
                        {
                            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                &mut arg_str,
                            );
                        }
                    }
                }

                self.apply_registry_borrow_to_call_arg(
                    &mut arg_str,
                    arg_to_generate,
                    type_name.as_deref(),
                    method,
                    i,
                    Some(arguments.len()),
                );

                let legacy_sig = call_site_sig
                    .as_ref()
                    .or(method_signature.as_ref())
                    .or(effective_sig.as_ref());
                let legacy_pidx = legacy_sig.map(|sig| sig.arg_param_index(i)).unwrap_or(i);
                let receiver_rt = type_name
                    .as_deref()
                    .or(receiver_type_name)
                    .or(self.current_struct_name.as_deref());
                let wants_ref = legacy_sig.is_some_and(|sig| {
                    crate::ir::signature_bridge::call_site_expects_shared_borrow(
                        sig, legacy_pidx,
                    )
                });
                let wants_owned = legacy_sig.is_some_and(|sig| {
                    crate::ir::signature_bridge::call_site_expects_owned_pass(sig, legacy_pidx)
                });

                if let Expression::Identifier { name, .. } = arg_to_generate {
                    if self.current_fn_mixed_forwarder_params.contains(name)
                        && crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(
                            object,
                        )
                    {
                        let mixed_forward_ref = self.in_if_condition;
                        if wants_ref
                            && !arg_str.starts_with('&')
                            && !self.callee_call_uses_rust_auto_borrow_for_owned_struct(arg_to_generate)
                            && !wants_owned
                        {
                            arg_str = format!("&{arg_str}");
                        } else if mixed_forward_ref && !arg_str.starts_with('&') {
                            arg_str = format!("&{arg_str}");
                        } else if wants_owned
                            && !mixed_forward_ref
                            && !self.current_fn_forward_ref_if_params.contains(name)
                            && arg_str.starts_with('&')
                            && !arg_str.starts_with("&mut ")
                        {
                            let base = arg_str.trim_start_matches('&');
                            arg_str = if base.ends_with(".clone()") {
                                base.to_string()
                            } else {
                                format!("{base}.clone()")
                            };
                        }
                    }
                    self.apply_forward_ref_and_mixed_forwarder_call_coercion(
                        &mut arg_str,
                        arg_to_generate,
                        Some(object),
                        wants_ref,
                        wants_owned,
                    );
                }

                // Final int→float cast after all arg mutations (binary inner literal casts).
                {
                    let cast_sig = call_site_sig
                        .as_ref()
                        .or(method_signature.as_ref())
                        .or(effective_sig.as_ref());
                    if let Some(sig) = cast_sig {
                        let qualified_key = type_name
                            .as_ref()
                            .map(|tn| format!("{}::{}", tn, method));
                        let skip_cast = self.should_skip_int_to_float_auto_cast_with_global(
                            type_name.as_deref(),
                            method,
                            qualified_key.as_deref(),
                        );
                        if !skip_cast {
                            let pidx = sig.arg_param_index(i);
                            let param_ty = sig
                                .param_type_for_arg(i)
                                .or_else(|| sig.formal_param_type(pidx))
                                .or_else(|| sig.param_types.get(pidx));
                            if let Some(param_ty) = param_ty {
                                let arg_ty = self.infer_expression_type(arg_to_generate);
                                crate::codegen::rust::type_classification_utilities::maybe_cast_int_arg_to_float(
                                    &mut arg_str,
                                    arg_to_generate,
                                    param_ty,
                                    arg_ty.as_ref(),
                                );
                            }
                        }
                    }
                }

                let contract_sig = receiver_rt.and_then(|rt| {
                    self.resolve_method_function_signature(rt, method, arguments.len())
                        .or_else(|| {
                            self.lookup_method_signature(rt, method)
                                .map(|ms| ms.to_function_signature())
                        })
                });
                if let Some(mut reg_sig) = contract_sig {
                    if let Some(rt) = receiver_rt {
                        let qualified = format!("{rt}::{method}");
                        if let Some(reg) = self
                            .signature_registry
                            .get_signature(&qualified)
                            .or_else(|| self.get_signature_with_global(&qualified))
                        {
                            crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                                &mut reg_sig, reg,
                            );
                        }
                    }
                    let pidx = reg_sig.arg_param_index(i);
                    if !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &reg_sig, pidx,
                    ) {
                        let expected =
                            crate::ir::signature_bridge::safety_type_from_signature_param(
                                &reg_sig, pidx,
                            );
                        if !matches!(
                            expected.ownership,
                            crate::ir::safety_type::OwnedType::Ref(_)
                                | crate::ir::safety_type::OwnedType::MutRef(_)
                        ) {
                            self.finalize_owned_outer_formal_call_arg(
                                &mut arg_str,
                                arg_to_generate,
                                wants_ref,
                                wants_owned,
                            );
                        }
                    }
                    self.enforce_call_site_ownership_contract(
                        &mut arg_str,
                        arg_to_generate,
                        &reg_sig,
                        pidx,
                    );
                } else {
                    self.finalize_owned_outer_formal_call_arg(
                        &mut arg_str,
                        arg_to_generate,
                        wants_ref,
                        wants_owned,
                    );
                }
                self.maybe_pure_forwarding_strip_call_arg(
                    &mut arg_str,
                    arg_to_generate,
                    receiver_rt.as_deref(),
                    Some(method),
                    Some(i),
                    Some(arguments.len()),
                );

                if (self.callee_call_uses_rust_auto_borrow_for_owned_struct(arg_to_generate)
                    || legacy_sig.is_some_and(|sig| {
                        crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig,
                            legacy_pidx,
                        ) || crate::ir::signature_bridge::call_site_expects_owned_pass(
                            sig, legacy_pidx,
                        )
                    }))
                    && arg_str.starts_with('&')
                    && !arg_str.starts_with("&mut ")
                {
                    arg_str = arg_str.trim_start_matches('&').to_string();
                }

                arg_str
            })
            .collect();

        (args_vec, prev_float_target)
    }

    fn is_enum_variant_or_constructor(expr: &Expression) -> bool {
        match expr {
            Expression::StructLiteral { .. } => true,
            Expression::Identifier { name, .. } => Self::is_enum_variant_qualified_path(name),
            Expression::Call {
                function,
                arguments,
                ..
            } if arguments.is_empty() => {
                if let Expression::FieldAccess { object, .. } = &**function {
                    matches!(&**object, Expression::Identifier { name, .. } if name.chars().next().is_some_and(|c| c.is_uppercase()))
                } else {
                    false
                }
            }
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name.chars().next().is_some_and(|c| c.is_uppercase()))
            }
            _ => false,
        }
    }

    /// Returns true if the type is a collection whose element/key type is String.
    /// Covers Vec<String>, HashMap<String, _>, HashSet<String>, etc.
    fn is_string_element_collection(ty: Option<&Type>) -> bool {
        let Some(t) = ty else { return false };
        match t {
            Type::Vec(inner) => crate::codegen::rust::types::is_windjammer_text_type(inner),
            Type::Parameterized(name, args) => {
                if args.is_empty() {
                    return false;
                }
                let is_known_collection = matches!(
                    name.as_str(),
                    "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet" | "VecDeque" | "LinkedList"
                );
                is_known_collection
                    && crate::codegen::rust::types::is_windjammer_text_type(&args[0])
            }
            _ => false,
        }
    }

    /// Fallback: when `infer_expression_type` returns an unparameterized `Custom("HashMap")`
    /// etc., check if the function return type matches a known String-keyed collection and
    /// the receiver type name matches.
    fn is_string_collection_from_return_type(
        receiver_ty: Option<&Type>,
        return_ty: Option<&Type>,
    ) -> bool {
        let Some(ret) = return_ty else { return false };
        let receiver_name = match receiver_ty {
            Some(Type::Custom(n)) => Some(n.as_str()),
            None => None,
            _ => return false,
        };
        match ret {
            Type::Vec(inner) if receiver_name.is_none() || receiver_name == Some("Vec") => {
                crate::codegen::rust::types::is_windjammer_text_type(inner)
            }
            Type::Parameterized(name, args) if !args.is_empty() => {
                let name_matches = receiver_name.is_none() || receiver_name == Some(name.as_str());
                let is_known_collection = matches!(
                    name.as_str(),
                    "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet" | "VecDeque" | "LinkedList"
                );
                name_matches
                    && is_known_collection
                    && crate::codegen::rust::types::is_windjammer_text_type(&args[0])
            }
            _ => false,
        }
    }
}
