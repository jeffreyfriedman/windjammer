//! Method-call argument codegen.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// When `receiver.method(args)` is a primitive float method, set literal/coercion context
    /// for arguments to match the receiver float type (`f32` vs `f64`).
    pub(in crate::codegen::rust) fn push_float_method_argument_context(
        &mut self,
        method: &str,
        object: &Expression<'ast>,
    ) -> Option<Type> {
        let prev = self.assignment_float_target_type.clone();
        let receiver_type_inferred = self.infer_expression_type(object);
        let stdlib = crate::analyzer::SignatureRegistry::stdlib();
        let preserves_on = |float: &str| {
            crate::analyzer::stdlib_method_traits::method_preserves_float_receiver(
                method,
                Some(&Type::Custom(float.to_string())),
                stdlib,
            )
        };
        // Primitive float methods are registered on both f32 and f64; struct builders
        // like `Slider::max` must not match (receiver is not a float type).
        let is_primitive_float_method = preserves_on("f32") && preserves_on("f64");
        let receiver_is_float = receiver_type_inferred.as_ref().is_some_and(|rty| {
            crate::codegen::rust::type_classification_utilities::is_float_type(rty)
                || matches!(rty, Type::Custom(n) if n == "float")
        });
        let is_float_method =
            is_primitive_float_method && (receiver_is_float || receiver_type_inferred.is_none());
        if is_float_method {
            use crate::type_inference::FloatType;
            let from_numeric =
                self.numeric_inference
                    .as_ref()
                    .and_then(|ni| match ni.get_float_type(object) {
                        FloatType::F32 => Some(Type::Custom("f32".to_string())),
                        FloatType::F64 => Some(Type::Custom("f64".to_string())),
                        FloatType::Unknown => None,
                    });
            if let Some(ty) = from_numeric {
                self.assignment_float_target_type = Some(ty);
            } else if let Some(float_name) = receiver_type_inferred
                .as_ref()
                .and_then(crate::analyzer::stdlib_method_traits::float_primitive_name)
            {
                self.assignment_float_target_type = Some(Type::Custom(float_name.to_string()));
            } else if let Some(ref rft) = receiver_type_inferred {
                match rft {
                    Type::Custom(n) if matches!(n.as_str(), "f64" | "float" | "f32") => {
                        let suffix = if n == "f32" { "f32" } else { "f64" };
                        self.assignment_float_target_type = Some(Type::Custom(suffix.to_string()));
                    }
                    Type::Float => {
                        // Windjammer default float is f32 unless inference/context says f64.
                        self.assignment_float_target_type = Some(Type::Custom("f32".to_string()));
                    }
                    _ => {}
                }
            } else {
                self.assignment_float_target_type = Some(Type::Custom("f32".to_string()));
            }
        }
        prev
    }

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
        let prev_float_target = self.push_float_method_argument_context(method, object);
        let receiver_type_inferred = self.infer_expression_type(object);
        // `Vec<f64>` / `Vec<f32>` receiver → float element type for generic store formals (`push(T)`).
        let collection_float_elem: Option<Type> = {
            let receiver_ty = receiver_type_inferred.clone().or_else(|| match object {
                Expression::Identifier { name, .. } => self
                    .local_var_types
                    .get(name.as_str())
                    .cloned()
                    .or_else(|| {
                        self.current_function_params
                            .iter()
                            .find(|p| p.name == *name)
                            .map(|p| p.type_.clone())
                    }),
                _ => None,
            });
            receiver_ty.as_ref().and_then(|rty| {
                Self::peeled_collection_element_type(rty)
                    .filter(|e| {
                        crate::codegen::rust::type_classification_utilities::is_float_type(e)
                    })
                    .cloned()
            })
        };
        if self.assignment_float_target_type.is_none() {
            if let Some(ref elem) = collection_float_elem {
                self.assignment_float_target_type = Some(elem.clone());
            }
        }

        let args_vec: Vec<String> = arguments
            .iter()
            .enumerate()
            .map(|(i, (_label, arg))| {
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
                    let strip_ref_for_collection_key = sig_for_effective
                        .is_some_and(|sig| {
                            crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
                                sig,
                                i,
                                receiver_type_name,
                            )
                        })
                        || crate::codegen::rust::stdlib_method_traits::method_arg_expects_borrowed_reference_qualified(
                            method,
                            receiver_type_name,
                            &self.signature_registry,
                            i,
                        );

                    if strip_ref_for_collection_key {
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
                            .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Owned))
                            || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                sig, sig_param_idx,
                            )
                            || sig.formal_param_type(sig_param_idx).is_some_and(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                                    && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                        sig, sig_param_idx,
                                    )
                            });
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
                    if crate::codegen::rust::stdlib_method_traits::method_predicate_closure_receives_ref(
                        method,
                    ) {
                        if let Expression::Closure { parameters, .. } = arg_to_generate {
                            let mut added = Vec::new();
                            for p in parameters.iter() {
                                let name = Self::closure_param_binding_name(p);
                                if !self.borrowed_iterator_vars.contains(&name) {
                                    self.borrowed_iterator_vars.insert(name.clone());
                                    added.push(name);
                                }
                            }
                            added
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };

                let prev_closure_predicate = self.closure_predicate_typed_params;
                if crate::codegen::rust::stdlib_method_traits::method_predicate_closure_receives_ref(
                    method,
                ) {
                    self.closure_predicate_typed_params = true;
                }

                let prev_arg_float_target = self.assignment_float_target_type.clone();
                let prev_call_arg_expected = self.call_arg_expected_type.clone();
                // Prefer specialized call-site signature (e.g. Vec<(String,String)>::push)
                // so nested tuple slots get owned-text coercion.
                if let Some(sig) = sig_for_effective {
                    let mut specialized = sig.clone();
                    if let Some(recv_ty) = crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                        receiver_type_name,
                        receiver_type_inferred.as_ref(),
                        self.current_function_return_type.as_ref(),
                    ) {
                        crate::codegen::rust::stdlib_signature_specialization::specialize_signature_for_receiver(
                            &mut specialized,
                            &recv_ty,
                        );
                    }
                    let pidx = specialized.arg_param_index(i);
                    let mut param_ty = specialized
                        .param_type_for_arg(i)
                        .or_else(|| specialized.formal_param_type(pidx))
                        .or_else(|| specialized.param_types.get(pidx))
                        .cloned();
                    // Unspecialized generic store formal (`T`) → concrete collection element.
                    if param_ty
                        .as_ref()
                        .is_some_and(|t| matches!(t, Type::Custom(n) if n == "T" || n == "E" || n == "V"))
                    {
                        if let Some(elem) = receiver_type_inferred
                            .as_ref()
                            .and_then(|rty| Self::peeled_collection_element_type(rty))
                            .cloned()
                        {
                            param_ty = Some(elem);
                        }
                    }
                    if param_ty.as_ref().is_some_and(
                        crate::codegen::rust::type_classification_utilities::is_float_type,
                    ) && self.assignment_float_target_type.is_none()
                    {
                        self.assignment_float_target_type = param_ty.clone();
                    }
                    if let Some(ty) = param_ty {
                        self.call_arg_expected_type = Some(ty);
                    }
                }

                let scope = self.arg_gen_scope();
                let mut arg_str = self.generate_expression(arg_to_generate);
                self.closure_predicate_typed_params = prev_closure_predicate;
                self.restore_arg_gen_scope(scope);
                self.suppress_borrowed_clone = prev_suppress;
                self.assignment_float_target_type = prev_arg_float_target;
                self.call_arg_expected_type = prev_call_arg_expected;
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
                let recv_for_wrap = receiver_type_name
                    .map(str::to_string)
                    .or_else(|| self.mc_infer_method_receiver_type_name(object));
                let closure_taking = crate::codegen::rust::stdlib_method_traits::method_is_closure_taking_qualified(
                    method,
                    recv_for_wrap.as_deref(),
                    &self.signature_registry,
                ) || crate::analyzer::stdlib_method_traits::consensus_closure_taking_method(
                    method,
                    &crate::analyzer::SignatureRegistry::stdlib(),
                );
                if i == 0 && closure_taking {
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
                    // Never invent a receiver type from a unique stdlib method-name hit
                    // (`join` → `Vec`). That mis-binds `strings::join` to `Vec::join` and
                    // owns `&str` literals. Prefer inferred/sig receivers only.
                    let object_is_runtime_std_module = matches!(
                        object,
                        Expression::Identifier { name, .. }
                            if self.is_imported_runtime_std_module(name)
                    );
                    let receiver_for_ir = if object_is_runtime_std_module {
                        None
                    } else {
                        receiver_type_name
                            .map(str::to_string)
                            .or_else(|| self.self_field_access_receiver_type_name(object))
                            .or_else(|| self.mc_infer_method_receiver_type_name(object))
                            .or_else(|| self.infer_type_name(object))
                            .or_else(|| {
                                self.infer_expression_type(object)
                                    .and_then(|t| Self::type_to_name(&t))
                            })
                            .or_else(|| {
                                sig_for_effective.as_ref().and_then(|sig| {
                                    crate::codegen::rust::stdlib_method_traits::receiver_type_from_qualified_sig(sig)
                                        .map(str::to_string)
                                })
                            })
                    };
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
                        receiver_for_ir.as_deref(),
                        Some(arguments.len()),
                    ) {
                        // Collection-key finalize, vec-local borrow, mixed-forwarder,
                        // owned-outer, and reuse-clone live in terminal IR reconcile.
                        let fallback_sig = sig_for_effective
                            .cloned()
                            .or_else(|| method_signature.clone())
                            .unwrap_or_default();
                        let receiver_rt = receiver_for_ir.as_deref().or(receiver_type_name).or_else(
                            || {
                                if matches!(
                                    object,
                                    Expression::Identifier { name, .. }
                                        if name == "self" || name == "Self"
                                ) {
                                    self.current_struct_name.as_deref()
                                } else {
                                    None
                                }
                            },
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
                        coerced = self.normalize_owned_copy_match_binding_call_arg(
                            arg_to_generate,
                            &coerced,
                            &contract_sig,
                            i,
                        );
                        coerced = self.maybe_wrap_fn_pointer_callback_bridge(
                            arg_to_generate,
                            &coerced,
                        );
                        // Terminal IR reconcile owns prefer-shared enforce, copy-aggregate
                        // peel, mixed-forwarder / owned-outer, match-arm text, pattern/`&str`,
                        // runtime-std borrow, stub auto-own, and shared-ref strip.
                        // Use `qualified_callee` (same key as apply_ir) so runtime fallback
                        // lookup finds `Connection::query`, not bare `query`.
                        self.reconcile_post_ir_mut_borrow_and_owned_peel(
                            &mut coerced,
                            arg_to_generate,
                            &qualified_callee,
                            i,
                            &contract_sig,
                            &self.signature_registry,
                            receiver_rt.as_deref(),
                            Some(object),
                            Some(arguments.len()),
                            false,
                        );
                        let borrowed_text = self
                            .inferred_borrowed_params
                            .iter()
                            .chain(self.str_ref_optimized_params.iter())
                            .cloned()
                            .collect::<std::collections::HashSet<_>>();
                        coerced = crate::codegen::rust::string_utilities::finalize_explicit_user_clone_call_site(
                            arg_to_generate,
                            &arg_str,
                            &coerced,
                            Some(&contract_sig),
                            i,
                            &borrowed_text,
                            &self.current_function_params,
                        );
                        return coerced;
                    }
                    debug_assert!(
                        false,
                        "IR call-site coercion must be total when call_sites is on ({qualified_callee})"
                    );
                    // Phase 5: never fall through to legacy method-arg ownership path.
                    return arg_str;
                }

                arg_str
            })
            .collect();

        (args_vec, prev_float_target)
    }
}
