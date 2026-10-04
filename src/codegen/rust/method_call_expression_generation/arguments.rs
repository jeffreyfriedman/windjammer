//! Method-call argument codegen.

use crate::analyzer::{OwnershipMode, SignatureRegistry};
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
        // Only apply float literal context when the receiver is known float — never when
        // inference is absent (i32 `.max(-100).min(100)` must not cast bounds to f32).
        let is_float_method = is_primitive_float_method && receiver_is_float;
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

        let mut args_vec: Vec<String> = arguments
            .iter()
            .enumerate()
            .map(|(i, (_label, arg))| {
                let receiver_type_name_owned = type_name
                    .clone()
                    .or_else(|| self.mc_infer_method_receiver_type_name(object))
                    .or_else(|| {
                        receiver_type_inferred
                            .as_ref()
                            .and_then(|t| Self::type_to_name(t))
                    })
                    .or_else(|| {
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
                let is_external_module_method = match object {
                    Expression::Identifier { name, .. } => {
                        name.chars().next().is_some_and(|c| c.is_lowercase())
                            && !self.current_function_params.iter().any(|p| p.name == *name)
                            && !self.local_var_types.contains_key(name)
                            && !self.match_arm_bindings.contains(name.as_str())
                    }
                    _ => false,
                };
                let external_module_mut_reborrow = is_external_module_method
                    && i == 0
                    && matches!(
                        arg,
                        Expression::Identifier { name, .. }
                            if self.inferred_mut_borrowed_params.contains(name)
                    );

                let mut call_site_sig = self.mc_select_call_site_signature(
                    object,
                    method,
                    arguments,
                    method_signature,
                );
                // Defining-module `&str` demotion may only exist on the global registry
                // after that file's codegen (`emitted_rust_ref_params`). Merge before IR
                // coercion so FieldAccess args (`req.path`) borrow instead of moving.
                // Match bindings (`Ok(mut app)`) often lack an inferred receiver type —
                // still refresh via the selected signature's qualified name (`App::handle`).
                if let Some(ref mut sig) = call_site_sig {
                    let mut keys: Vec<String> = Vec::new();
                    if let Some(rt) = receiver_type_name {
                        keys.push(format!("{rt}::{method}"));
                    }
                    if !sig.name.is_empty() {
                        keys.push(sig.name.clone());
                    }
                    if keys.is_empty() {
                        keys.push(method.to_string());
                    }
                    crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                        sig,
                        &self.signature_registry,
                        &keys,
                    );
                    if let Some(global) = self.global_signature_registry.as_ref() {
                        crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                            sig, global, &keys,
                        );
                    }
                }

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
                            self.is_collection_key_lookup_at_site(
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
                let prev_arg_int_target = self.assignment_int_target_type.clone();
                let prev_call_arg_expected = self.call_arg_expected_type.clone();
                // Prefer specialized call-site signature (e.g. Vec<(String,String)>::push)
                // so nested tuple slots get owned-text coercion.
                let mut method_param_ty: Option<Type> = None;
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
                    method_param_ty = param_ty.clone();
                    if let Some(ty) = param_ty {
                        self.call_arg_expected_type = Some(ty);
                    }
                }
                // P3.366/P3.367: Drive int literal suffixes from formals / receiver — override
                // file-wide i32-coord context (`seed.wrapping_mul(1103515245)` → `_u32` not `_i32`).
                if let Some(peer) = crate::codegen::rust::type_casting::assignment_int_peer_from_formal(
                    method_param_ty.as_ref(),
                )
                .or_else(|| {
                    crate::codegen::rust::type_casting::assignment_int_peer_from_formal(
                        receiver_type_inferred.as_ref(),
                    )
                })
                .or_else(|| {
                    receiver_type_name.and_then(|n| {
                        crate::codegen::rust::type_casting::assignment_int_peer_from_owner_type_name(n)
                    })
                })
                .or_else(|| {
                    use crate::type_inference::IntType;
                    match self.int_type_for_mixed_int_codegen(object) {
                        IntType::U32 => Some(Type::Uint),
                        IntType::I32 => Some(Type::Int32),
                        IntType::I64 => Some(Type::Int),
                        _ => None,
                    }
                }) {
                    self.assignment_int_target_type = Some(peer);
                }

                let scope = self.arg_gen_scope();
                let mut arg_str = self.generate_expression(arg_to_generate);
                self.closure_predicate_typed_params = prev_closure_predicate;
                self.restore_arg_gen_scope(scope);
                self.suppress_borrowed_clone = prev_suppress;
                self.assignment_float_target_type = prev_arg_float_target;
                self.assignment_int_target_type = prev_arg_int_target;
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
                        // Start from the IR/mc_resolve signature (`fallback_sig`). Replacing
                        // it with `resolve_method_function_signature` reintroduced bare leaf
                        // analysis stubs (`info` Borrowed + `Reference(str)`, no emit flags)
                        // that peeled `String::from("a")` back to `"a"` (P3.602 Logger).
                        let mut contract_sig = fallback_sig.clone();
                        if contract_sig.emitted_rust_ref_params.is_none() {
                            if let Some(resolved) = receiver_rt.as_deref().and_then(|rt| {
                                self.resolve_method_function_signature(
                                    rt,
                                    method,
                                    arguments.len(),
                                )
                            }) {
                                // Only adopt when it is not weaker than the IR sig.
                                let adopt = crate::codegen::rust::signature_promotion::codegen_refreshed_beats_analysis_only(
                                    &resolved,
                                    &contract_sig,
                                ) || resolved.emitted_rust_ref_params.is_some();
                                if adopt {
                                    contract_sig = resolved;
                                }
                            }
                        }
                        if object_is_runtime_std_module {
                            // No receiver type (`strings.starts_with`) — still refresh from
                            // the qualified runtime key so AsRef/`&str` beats WJ owned stubs.
                            if let Some(refreshed) = self.refresh_call_site_signature_for_arg(
                                Some(contract_sig.clone()),
                                &qualified_callee,
                                i,
                            ) {
                                contract_sig = refreshed;
                            }
                        } else if let Some(rt) = receiver_rt.as_deref() {
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
                        let pidx_after = contract_sig.arg_param_index(i);
                        let callee_wants_shared = crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                            &contract_sig, pidx_after,
                        ) || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(
                            &contract_sig, pidx_after,
                        ) || contract_sig
                            .param_types
                            .get(pidx_after)
                            .is_some_and(|t| matches!(t, Type::Reference(_)));
                        if callee_wants_shared && coerced.ends_with(".clone()") {
                            let base = coerced.trim_end_matches(".clone()").trim();
                            coerced = if base.starts_with('&') {
                                base.to_string()
                            } else {
                                format!("&{base}")
                            };
                        }
                        coerced = crate::codegen::rust::string_utilities::finalize_explicit_user_clone_call_site(
                            arg_to_generate,
                            &arg_str,
                            &coerced,
                            Some(&contract_sig),
                            i,
                            &self.emitted_rust_ref_formals,
                            &self.current_function_params,
                        );
                        coerced = crate::codegen::rust::call_site_borrow::reconcile_explicit_user_clone_into_owned_vec_formal(
                            self,
                            arg_to_generate,
                            coerced,
                            &contract_sig,
                            i,
                        );
                        coerced = crate::codegen::rust::call_site_borrow::normalize_explicit_deref_copy_operand(
                            arg_to_generate,
                            &coerced,
                        );
                        crate::codegen::rust::call_site_borrow::reconcile_method_call_owned_copy_scalar_identifier_arg(
                            self,
                            &mut coerced,
                            arg_to_generate,
                            &contract_sig,
                            i,
                            receiver_rt.as_deref(),
                        );
                        // P3.638: owned String formal must move FieldAccess (`meta.2`), not `&meta.2`.
                        // P3.636: field behind `&mut self` into owned formal must `.clone()`.
                        if matches!(arg_to_generate, Expression::FieldAccess { .. }) {
                            let pidx = contract_sig.arg_param_index(i);
                            // Prefer runtime `json::to_string(value: T)` owned contract over
                            // MutBorrowed `to_string` homonyms (String::to_string).
                            let std_owned = crate::analyzer::SignatureRegistry::stdlib()
                                .get_signature(&qualified_callee)
                                .or_else(|| {
                                    crate::analyzer::SignatureRegistry::stdlib()
                                        .get_signature("json::to_string")
                                })
                                .or_else(|| {
                                    crate::analyzer::SignatureRegistry::stdlib()
                                        .get_signature("json::to_string_pretty")
                                })
                                .filter(|std_sig| {
                                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                        std_sig,
                                        std_sig.arg_param_index(i),
                                    )
                                });
                            if let Some(std_sig) = std_owned {
                                if matches!(method, "to_string" | "to_string_pretty")
                                    && (qualified_callee.contains("json::")
                                        || matches!(
                                            object,
                                            Expression::Identifier { name, .. } if name == "json"
                                        ))
                                {
                                    contract_sig = std_sig.clone();
                                }
                            }
                            let pidx = contract_sig.arg_param_index(i);
                            let owned_text_slot = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                &contract_sig, pidx,
                            ) || (contract_sig
                                .formal_param_type(pidx)
                                .or_else(|| contract_sig.param_types.get(pidx))
                                .is_some_and(|t| {
                                    let bare = match t {
                                        Type::Reference(inner) | Type::MutableReference(inner) => {
                                            inner.as_ref()
                                        }
                                        other => other,
                                    };
                                    crate::codegen::rust::types::is_windjammer_text_type(bare)
                                        || matches!(bare, Type::Custom(_) | Type::Parameterized(_, _))
                                            && !self.is_type_copy(bare)
                                })
                                && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                    &contract_sig, pidx,
                                )
                                && !crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(
                                    &contract_sig, pidx,
                                ));
                            let behind_ref = self
                                .field_access_root_is_behind_reference(arg_to_generate);
                            // Multipass may emit `&mut self` without seeding inferred_*;
                            // still clone `self.field` into owned json::to_string.
                            let self_field = matches!(
                                arg_to_generate,
                                Expression::FieldAccess { object, .. }
                                    if matches!(
                                        &**object,
                                        Expression::Identifier { name, .. } if name == "self"
                                    )
                            ) && self.current_method_self_emits_borrowed_receiver();
                            if owned_text_slot {
                                if behind_ref || self_field {
                                    if !coerced.ends_with(".clone()")
                                        && !coerced.ends_with(".to_owned()")
                                    {
                                        let base = if coerced.starts_with('&')
                                            && !coerced.starts_with("&mut ")
                                        {
                                            coerced[1..].to_string()
                                        } else {
                                            coerced.clone()
                                        };
                                        coerced = format!("{base}.clone()");
                                    }
                                } else if coerced.starts_with('&')
                                    && !coerced.starts_with("&mut ")
                                {
                                    coerced = coerced[1..].to_string();
                                }
                            }
                        }
                        // P3.637: owned String locals into demoted `&str` method formals.
                        if let Expression::Identifier { name, .. } = arg_to_generate {
                            let pidx = contract_sig.arg_param_index(i);
                            let wants_str = contract_sig
                                .formal_param_type(pidx)
                                .or_else(|| contract_sig.param_types.get(pidx))
                                .is_some_and(
                                    crate::codegen::rust::string_utilities::param_is_rust_str_ref,
                                )
                                || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                    &contract_sig, pidx,
                                )
                                || (crate::ir::formal_predicates::formal_is_plain_windjammer_string(
                                    &contract_sig, pidx,
                                ) && matches!(
                                    contract_sig.param_ownership.get(pidx),
                                    Some(OwnershipMode::Borrowed)
                                ) && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    &contract_sig, pidx,
                                ));
                            if wants_str
                                && !coerced.starts_with('&')
                                && !coerced.starts_with("&mut ")
                                && !self.identifier_binding_already_rust_ref(name)
                                && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    &contract_sig, pidx,
                                )
                            {
                                coerced = format!("&{coerced}");
                            }
                        }
                        let pidx = contract_sig.arg_param_index(i);
                        let formal_copy = contract_sig
                            .formal_param_type(pidx)
                            .or_else(|| contract_sig.param_types.get(pidx))
                            .is_some_and(|t| {
                                let bare = match t {
                                    crate::parser::Type::Reference(inner)
                                    | crate::parser::Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                            });
                        if formal_copy && coerced.ends_with(".clone()") {
                            coerced = coerced.trim_end_matches(".clone()").to_string();
                        }
                        self.finalize_post_ir_collection_key_arg(
                            &mut coerced,
                            arg_to_generate,
                            i,
                            method,
                            &qualified_callee,
                            receiver_rt.as_deref(),
                            &contract_sig,
                        );
                        if crate::codegen::rust::call_site_borrow::user_wrote_explicit_deref(
                            arg_to_generate,
                        ) && coerced.starts_with('&')
                            && !coerced.starts_with("&mut ")
                        {
                            coerced = coerced[1..].to_string();
                        }
                        // Copy field projections into `&Copy` slots: rustc auto-borrows.
                        // Do not require `formal_copy` from a possibly-stale contract sig —
                        // `expression_is_copy` is the source of truth (auto_ref_deref_copy).
                        if matches!(arg_to_generate, Expression::FieldAccess { .. })
                            && self.expression_is_copy(arg_to_generate)
                            && coerced.starts_with('&')
                            && !coerced.starts_with("&mut ")
                        {
                            coerced = coerced[1..].to_string();
                        }
                        if let Expression::Identifier { name, .. } = arg_to_generate {
                            let is_ck = self.is_collection_key_lookup_at_site(
                                &contract_sig,
                                i,
                                receiver_rt.as_deref(),
                            );
                            // P3.635: peel `&label` into owned Copy formals — never undo
                            // confirmed HashMap/BTreeMap key borrows. Bare `get`/`remove`
                            // are not map keys (P3.649–651).
                            if !is_ck
                                && self.binding_is_copy_pass_by_value_scalar(name)
                                && coerced.starts_with('&')
                                && !coerced.starts_with("&mut ")
                            {
                                let slot_is_owned_copy_scalar =
                                    crate::codegen::rust::call_site_borrow::callee_user_arg_bare_formal_is_copy_pass_by_value(
                                        &contract_sig,
                                        i,
                                    );
                                if slot_is_owned_copy_scalar {
                                    let base = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                        &coerced,
                                    );
                                    if base == name.as_str() {
                                        coerced = name.clone();
                                    }
                                }
                            }
                        }
                        // P3.654–657 terminal: runtime-std `&str` must survive peel/strip
                        // when layered WJ stubs keep owned `string` formals.
                        if !coerced.starts_with('&')
                            && !coerced.starts_with("&mut ")
                            && crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                                &self.signature_registry,
                                &qualified_callee,
                                Some(&contract_sig),
                                i,
                            )
                            && matches!(
                                arg_to_generate,
                                Expression::Identifier { name, .. }
                                    if !self.emitted_rust_ref_formals.contains(name.as_str())
                                        && !self.str_ref_optimized_params.contains(name.as_str())
                            )
                        {
                            coerced = format!("&{coerced}");
                        }
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

        // P3.635 / WDB-134: HashMap/BTreeMap key args must emit `&key` (match scrutinees).
        // Only when the receiver is a map/set/wrapper — never blanketed `get`/`remove`
        // (P3.649–651: Vec::get / Vec::remove / Store::get take owned Copy indices).
        if crate::codegen::rust::stdlib_method_traits::is_map_key_method(method) {
            let receiver_rt_owned = type_name.clone().or_else(|| {
                self.mc_infer_method_receiver_type_name(object)
                    .or_else(|| self.infer_type_name(object))
                    .or_else(|| {
                        self.infer_expression_type(object)
                            .and_then(|t| Self::type_to_name(&t))
                    })
            });
            let receiver_rt = receiver_rt_owned.as_deref();
            let receiver_base = receiver_rt.map(|rt| rt.split('<').next().unwrap_or(rt));
            let receiver_is_mapish = receiver_base.is_some_and(|base| {
                crate::type_classification::is_map_type_name(base)
                    || crate::type_classification::is_set_type_name(base)
                    || crate::codegen::rust::stdlib_method_traits::is_map_deref_wrapper_type_name(
                        base,
                    )
                    // P3.660 / wj-sync: lock-guard bindings sometimes infer as the outer
                    // SharedMap alias rather than `MutexGuard<HashMap<…>>`.
                    || base == "SharedMap"
                    || base == "SharedMapSI"
                    || base.ends_with("SharedMap")
                    || base.ends_with("SharedMapSI")
            });
            // Field access with unknown type: still try collection-key finalize when
            // the resolved method signature already looks like a map-key borrow.
            let sig_looks_like_map_key = method_signature.as_ref().is_some_and(|sig| {
                self.is_collection_key_lookup_at_site(sig, 0, receiver_rt)
            });
            let wrapper_key_sig = crate::codegen::rust::stdlib_method_traits::hashmap_key_method_signature_for_wrapper(
                method,
                receiver_rt,
                &self.signature_registry,
            );
            // P3.660: enter when wrapper consensus finds HashMap::{get,contains_key}
            // even if the receiver name is a non-mapish alias / unknown guard.
            if receiver_is_mapish
                || (receiver_rt.is_none() && sig_looks_like_map_key)
                || wrapper_key_sig.is_some()
            {
                let qualified =
                    crate::codegen::rust::stdlib_method_traits::module_qualified_method_name(
                        receiver_rt,
                        object,
                        method,
                        |name| self.is_imported_runtime_std_module(name),
                    );
                let sig_for_key = method_signature
                    .clone()
                    .or(wrapper_key_sig)
                    .or_else(|| {
                        crate::codegen::rust::stdlib_method_traits::hashmap_key_method_signature_for_wrapper(
                            method,
                            receiver_rt,
                            &self.signature_registry,
                        )
                    });
                if let Some(sig_for_key) = sig_for_key {
                    for (i, arg_str) in args_vec.iter_mut().enumerate() {
                        let Some((_, arg_expr)) = arguments.get(i) else {
                            continue;
                        };
                        self.finalize_post_ir_collection_key_arg(
                            arg_str,
                            arg_expr,
                            i,
                            method,
                            &qualified,
                            receiver_rt,
                            &sig_for_key,
                        );
                    }
                }
            }
        }

        // P3.649–651: `Vec::get(0)` / `Vec::remove(0)` / user `Store::get(99)` take owned
        // Copy indices — peel stale `&N_usize` / `&99_i64` left by map-key over-borrow.
        if matches!(method, "get" | "contains_key" | "get_key_value" | "remove") {
            let receiver_rt_owned = type_name.clone().or_else(|| {
                self.mc_infer_method_receiver_type_name(object)
                    .or_else(|| self.infer_type_name(object))
                    .or_else(|| {
                        self.infer_expression_type(object)
                            .and_then(|t| Self::type_to_name(&t))
                    })
            });
            let receiver_base = receiver_rt_owned
                .as_deref()
                .map(|rt| rt.split('<').next().unwrap_or(rt));
            let receiver_is_mapish = receiver_base.is_some_and(|base| {
                crate::type_classification::is_map_type_name(base)
                    || crate::type_classification::is_set_type_name(base)
                    || crate::codegen::rust::stdlib_method_traits::is_map_deref_wrapper_type_name(
                        base,
                    )
                    || base == "SharedMap"
                    || base == "SharedMapSI"
                    || base.ends_with("SharedMap")
                    || base.ends_with("SharedMapSI")
            });
            if !receiver_is_mapish {
                // P3.660: SharedMap / MutexGuard consensus still map-key — do not peel
                // String keys when the inferred name is a non-mapish alias.
                if crate::codegen::rust::stdlib_method_traits::hashmap_key_method_signature_for_wrapper(
                    method,
                    receiver_rt_owned.as_deref(),
                    &self.signature_registry,
                )
                .is_some()
                {
                    // keep args
                } else {
                let peel_sig = method_signature.as_ref();
                for (i, arg_str) in args_vec.iter_mut().enumerate() {
                    if !arg_str.starts_with('&') || arg_str.starts_with("&mut ") {
                        continue;
                    }
                    let owned_copy = peel_sig.is_some_and(|sig| {
                        crate::codegen::rust::call_site_borrow::callee_user_arg_bare_formal_is_copy_pass_by_value(
                            sig, i,
                        )
                    }) || peel_sig.is_some_and(|sig| {
                        let pidx = sig.arg_param_index(i);
                        sig.formal_param_type(pidx)
                            .or_else(|| sig.param_types.get(pidx))
                            .is_some_and(|t| {
                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    && crate::type_classification::is_copy_pass_by_value_formal(t)
                            })
                    });
                    // Slice/Vec::get / remove: stdlib formals are owned usize.
                    let std_owned_usize = matches!(method, "get" | "remove")
                        && SignatureRegistry::stdlib()
                            .get_signature(&format!("Vec::{method}"))
                            .or_else(|| {
                                SignatureRegistry::stdlib()
                                    .get_signature(&format!("slice::{method}"))
                            })
                            .is_some_and(|sig| {
                                let pidx = sig.arg_param_index(i);
                                crate::type_classification::is_copy_pass_by_value_formal(
                                    sig.formal_param_type(pidx)
                                        .or_else(|| sig.param_types.get(pidx))
                                        .unwrap_or(&Type::Custom("_".into())),
                                )
                            });
                    if owned_copy || std_owned_usize {
                        *arg_str = crate::codegen::rust::expression_utilities::borrow_base_expr(
                            arg_str,
                        )
                        .to_string();
                    }
                }
                }
            }
        }

        (args_vec, prev_float_target)
    }
}
