//! Final Rust emission for method calls.

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub(in crate::codegen::rust) fn mc_finalize_method_call_expression(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
        type_args: &Option<Vec<Type>>,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        obj_str: String,
        args: Vec<String>,
        prev_float_target: Option<Type>,
    ) -> String {
        if std::env::var("WJ_DEBUG_FIND_PATTERN").is_ok() && method == "find" {}
        let mut obj_str = obj_str;
        let resolved_signature =
            self.mc_select_call_site_signature(object, method, arguments, method_signature);
        let receiver_type_name = self
            .mc_infer_method_receiver_type_name(object)
            .or_else(|| self.infer_type_name(object));
        let inferred_receiver_ty = self.infer_expression_type(object);
        let resolved_signature = resolved_signature.map(|mut sig| {
            if let Some(rt) = receiver_type_name.as_deref() {
                if let Some(reg) = self.resolve_method_function_signature_specialized(
                    rt,
                    method,
                    arguments.len(),
                    inferred_receiver_ty.as_ref(),
                ) {
                    crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                        &mut sig, &reg,
                    );
                }
                if let Some(ms) = self.lookup_method_signature(rt, method) {
                    let mut reg_sig = ms.to_function_signature();
                    if let Some(recv_ty) = crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                        Some(rt),
                        inferred_receiver_ty.as_ref(),
                        self.current_function_return_type.as_ref(),
                    ) {
                        crate::codegen::rust::stdlib_signature_specialization::specialize_signature_for_receiver(
                            &mut reg_sig,
                            &recv_ty,
                        );
                    }
                    use crate::codegen::rust::signature_promotion::{
                        converged_has_reference_params_over_bare,
                        method_registry_reflects_emitted_owned,
                    };
                    if converged_has_reference_params_over_bare(&sig, &reg_sig)
                        && !method_registry_reflects_emitted_owned(&sig)
                    {
                        sig = reg_sig;
                    }
                }
            }
            if let Some(recv_ty) =
                crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                    receiver_type_name.as_deref(),
                    inferred_receiver_ty.as_ref(),
                    self.current_function_return_type.as_ref(),
                )
            {
                crate::codegen::rust::stdlib_signature_specialization::specialize_signature_for_receiver(
                    &mut sig, &recv_ty,
                );
            }
            sig
        });
        // When IR call-sites are on, `mc_build_method_call_arg_strings` already ran
        // `apply_ir_call_site_coercion` + terminal `reconcile_post_ir_*`. Re-running the
        // ownership rewrite here is a third legacy layer (add/strip/re-add ping-pong).
        // Keep this path only for isolated unit tests that disable `call_sites`.
        let args = if self.ir_cutover.call_sites {
            args
        } else if let Some(ref sig) = resolved_signature {
            args.into_iter()
                .enumerate()
                .map(|(i, mut arg_str)| {
                    if arg_str.ends_with(".clone()") {
                        if let Some((_, arg_expr)) = arguments.get(i) {
                            if let Expression::Identifier { name, .. } = arg_expr {
                                let arg_is_fn_param = self
                                    .current_function_params
                                    .iter()
                                    .any(|p| p.name == *name);
                                let pidx = sig.arg_param_index(i);
                                let this_arg_expects_shared_borrow =
                                    !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                        &sig, pidx,
                                    )
                                        && (sig.param_types.get(pidx).is_some_and(|t| {
                                            matches!(
                                                t,
                                                Type::Reference(_)
                                                    | Type::MutableReference(_)
                                            )
                                        })
                                            || matches!(
                                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                                    &sig,
                                                    i,
                                                    receiver_type_name.as_deref(),
                                                ),
                                                OwnershipMode::Borrowed
                                            ));
                                if arg_is_fn_param && this_arg_expects_shared_borrow {
                                    crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                        &mut arg_str,
                                    );
                                }
                            }
                        }
                    }
                    if arg_str.contains("string_to_ffi(") {
                        return arg_str;
                    }
                    let sig_param_idx = sig.arg_param_index(i);
                    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, sig_param_idx,
                    ) && arg_str.starts_with('&')
                        && !arg_str.starts_with("&mut ")
                    {
                        return arg_str.trim_start_matches('&').to_string();
                    }
                    let mut ownership =
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                            sig, i, receiver_type_name.as_deref(),
                        );
                    if sig.param_types.get(sig_param_idx).is_some_and(|t| {
                        matches!(t, Type::MutableReference(_))
                    }) {
                        ownership = OwnershipMode::MutBorrowed;
                    } else if let Some(rt) = receiver_type_name.as_deref() {
                        if let Some(resolved) = self.resolve_method_function_signature(
                            rt,
                            method,
                            arguments.len(),
                        ) {
                            let ridx = resolved.arg_param_index(i);
                            if resolved.param_types.get(ridx).is_some_and(|t| {
                                matches!(t, Type::MutableReference(_))
                            }) {
                                ownership = OwnershipMode::MutBorrowed;
                            }
                        }
                    }
                    if crate::codegen::rust::call_signature_resolution::callee_user_arg_expects_mut_borrow(
                        &sig, i,
                    ) {
                        ownership = OwnershipMode::MutBorrowed;
                    } else if let Some(rt) = receiver_type_name.as_deref() {
                        let qualified = format!("{rt}::{method}");
                        if self
                            .function_emitted_mut_arg_indices
                            .get(&qualified)
                            .or_else(|| self.function_emitted_mut_arg_indices.get(method))
                            .is_some_and(|indices| indices.contains(&i))
                        {
                            ownership = OwnershipMode::MutBorrowed;
                        }
                    }
                    // Spurious `&mut` from stale IR call-site lowering: strip when the
                    // converged contract is owned (not MutBorrowed).
                    if matches!(ownership, OwnershipMode::Owned)
                        && arg_str.starts_with("&mut ")
                        && !matches!(
                            sig.param_types.get(sig_param_idx),
                            Some(Type::MutableReference(_))
                        )
                    {
                        arg_str = arg_str["&mut ".len()..].to_string();
                    }
                    // Signature-driven: owned String expression at an `&str` formal needs `&`.
                    // Preserves source `.to_string()` (i32→String) while satisfying push_str.
                    if !arg_str.starts_with('&')
                        && arg_str.ends_with(".to_string()")
                        && sig.param_type_for_arg(i).is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        })
                    {
                        arg_str = format!("&{arg_str}");
                    }
                    // After stripping stale `&mut`, owned WJ `string` formals still need
                    // string-literal → String coercion at the call site.
                    if matches!(ownership, OwnershipMode::Owned) {
                        if let Some((_, arg_expr)) = arguments.get(i) {
                            crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
                                Some(sig),
                                i,
                                Some(method),
                                arg_expr,
                                &mut arg_str,
                                receiver_type_name.as_deref(),
                                Some(&self.enum_variant_types),
                            );
                        }
                    }
                    let param_is_copy = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                        let bare = match t {
                            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                            other => other,
                        };
                        self.is_type_copy(bare)
                            && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                    });
                    let is_collection_key =
                        crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
                            sig,
                            i,
                            receiver_type_name.as_deref(),
                        );
                    let callee_arg_emits_owned = {
                        let from_sig = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            &sig, sig_param_idx,
                        );
                        from_sig
                            || receiver_type_name.as_ref().is_some_and(|rt| {
                                self.resolve_method_function_signature(
                                    rt,
                                    method,
                                    arguments.len(),
                                )
                                .is_some_and(|resolved| {
                                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                        &resolved,
                                        resolved.arg_param_index(i),
                                    )
                                })
                            })
                    };
                    let apply_borrow = |arg_str: &mut String| {
                        if callee_arg_emits_owned {
                            return;
                        }
                        let param_is_ref_type = sig.param_types.get(sig_param_idx).is_some_and(
                            |t| matches!(t, Type::Reference(_) | Type::MutableReference(_)),
                        );
                        if !param_is_ref_type
                            && matches!(
                                sig.param_ownership.get(sig_param_idx),
                                Some(OwnershipMode::Owned)
                            )
                            && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                &sig, sig_param_idx,
                            )
                        {
                            return;
                        }
                        if sig.formal_param_type(sig_param_idx).is_some_and(|t| {
                            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                && crate::codegen::rust::types::is_windjammer_text_type(t)
                        }) && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            &sig, sig_param_idx,
                        ) {
                            return;
                        }
                        let arg_is_string_literal = matches!(
                            arguments.get(i).map(|(_, e)| e),
                            Some(Expression::Literal {
                                value: Literal::String(_),
                                ..
                            })
                        );
                        // After .to_string() stripping, a MethodCall like "lit".to_string()
                        // becomes bare "lit" in arg_str. Check if arg_str is now a string
                        // literal — it's already &str, adding & would create &&str.
                        let arg_str_is_bare_literal = arg_str.starts_with('"')
                            || arg_str.starts_with("r\"")
                            || arg_str.starts_with("r#\"");
                        if arg_is_string_literal || arg_str_is_bare_literal
                            || (param_is_copy && !is_collection_key) {
                            return;
                        }
                        if let Some((_, arg_expr)) = arguments.get(i) {
                            let inner = match arg_expr {
                                Expression::Unary { op: UnaryOp::Ref, operand, .. } => operand,
                                other => other,
                            };
                            if let Expression::Identifier { name, .. } = inner {
                                if self.identifier_already_ref(name)
                                    || self.str_ref_optimized_params.contains(name.as_str())
                                    || self.emitted_rust_ref_formals.contains(name.as_str())
                                    || (self.inferred_borrowed_params.contains(name.as_str())
                                        && !self.collection_key_owned_params.contains(name.as_str())
                                        && self.current_function_params.iter().any(|p| {
                                            p.name == *name
                                                && crate::codegen::rust::types::is_windjammer_text_type(
                                                    &p.type_,
                                                )
                                        }))
                                {
                                    return;
                                }
                            }
                        }
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(arg_str);
                        if !arg_str.starts_with('&') {
                            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                arg_str,
                            );
                        }
                    };
                    let callee_formal_is_copy = sig.formal_param_type(sig_param_idx).is_some_and(|t| {
                        let bare = match t {
                            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                            other => other,
                        };
                        self.is_type_copy(bare)
                            && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                    });
                    arg_str = match ownership {
                        crate::analyzer::OwnershipMode::MutBorrowed
                            if !arg_str.starts_with("&mut ") =>
                        {
                            // Collection keys are always `&Q` (including `get_mut`); never `&mut Q`.
                            // Temporaries (`Type::new()`) cannot be mut-reborrowed as lvalues —
                            // shared `&` matches emitted `&T` formals after multipass convergence.
                            let arg_expr = arguments.get(i).map(|(_, e)| e);
                            let can_mut = arg_expr.is_some_and(|e| {
                                crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(
                                    e,
                                )
                            });
                            let defining_module_emits_mut =
                                receiver_type_name.as_deref().map_or(false, |rt| {
                                    let qualified = format!("{rt}::{method}");
                                    self.function_emitted_mut_arg_indices
                                        .get(&qualified)
                                        .or_else(|| self.function_emitted_mut_arg_indices.get(method))
                                        .is_some_and(|indices| indices.contains(&i))
                                });
                            if is_collection_key || !can_mut {
                                apply_borrow(&mut arg_str);
                                arg_str
                            } else if callee_formal_is_copy
                                && (callee_arg_emits_owned || !defining_module_emits_mut)
                            {
                                // Owned Copy aggregates pass by value. Defining-module `&mut T`
                                // emission (`Ability::activate`) still uses `&mut self.player`.
                                return arg_str;
                            } else if let Some(Expression::Identifier { name, .. }) = arg_expr {
                                if self.identifier_already_mut_ref(name) {
                                    return arg_str;
                                }
                                crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                    &mut arg_str,
                                );
                                if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                                    format!("&mut {}", arg_str.trim_start_matches('&'))
                                } else {
                                    format!("&mut {arg_str}")
                                }
                            } else {
                                crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                    &mut arg_str,
                                );
                                if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                                    format!("&mut {}", arg_str.trim_start_matches('&'))
                                } else {
                                    format!("&mut {arg_str}")
                                }
                            }
                        }
                        crate::analyzer::OwnershipMode::Borrowed
                            if !callee_arg_emits_owned =>
                        {
                            if callee_formal_is_copy && !is_collection_key {
                                if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                                    return arg_str.trim_start_matches('&').to_string();
                                }
                                return arg_str;
                            }
                            // Demote a stale `&mut` prefix when the formal is shared `&T`
                            // (multipass: MutBorrowed inference vs emitted `&QuestId`).
                            if arg_str.starts_with("&mut ") {
                                return format!("&{}", arg_str.trim_start_matches("&mut "));
                            }
                            if arg_str.starts_with('&') {
                                // Strip stale `&` when the binding is already a Rust shared ref
                                // (`key: &str` → `map.get(key)`, not `map.get(&key)`).
                                if let Some((_, arg_expr)) = arguments.get(i) {
                                    if let Expression::Identifier { name, .. } = arg_expr {
                                        if (self.emitted_rust_ref_formals.contains(name.as_str())
                                            || self.str_ref_optimized_params.contains(name.as_str())
                                            || self.identifier_already_ref(name))
                                            && !self.collection_key_owned_params.contains(name.as_str())
                                        {
                                            let base = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                                &arg_str,
                                            );
                                            if base == name.as_str() {
                                                return name.clone();
                                            }
                                        }
                                    }
                                }
                                return arg_str;
                            }
                            if !is_collection_key
                                && sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                    matches!(t, Type::String)
                                        || matches!(t, Type::Custom(n) if n == "string" || n == "String")
                                })
                            {
                                arg_str
                            } else {
                                apply_borrow(&mut arg_str);
                                arg_str
                            }
                        }
                        _ if sig.param_types.get(sig_param_idx).is_some_and(|t| {
                            matches!(t, Type::Reference(_))
                        }) && !arg_str.starts_with('&')
                            && (!callee_formal_is_copy || is_collection_key)
                            && !callee_arg_emits_owned =>
                        {
                            apply_borrow(&mut arg_str);
                            arg_str
                        }
                        crate::analyzer::OwnershipMode::Owned => {
                            let formal_is_owned_string = sig
                                .formal_param_type(sig_param_idx)
                                .is_some_and(|t| {
                                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                        && crate::codegen::rust::types::is_windjammer_text_type(t)
                                });
                            if formal_is_owned_string {
                                let caller_passes_str_slice =
                                    arguments.get(i).is_some_and(|(_, arg_expr)| {
                                        if let Expression::Identifier { name, .. } = *arg_expr {
                                            self.str_ref_optimized_params.contains(name.as_str())
                                                || self.inferred_borrowed_params.contains(name)
                                                || self.current_function_params.iter().any(|p| {
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
                                if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                                    return arg_str.trim_start_matches('&').to_string();
                                }
                            }
                            let param_is_str_ref = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                            });
                            if arg_str.starts_with('&')
                                && !arg_str.starts_with("&mut ")
                                && !param_is_str_ref
                                && !is_collection_key
                                && !sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                    matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                })
                                && !matches!(
                                    sig.param_ownership.get(sig_param_idx),
                                    Some(crate::analyzer::OwnershipMode::MutBorrowed)
                                        | Some(crate::analyzer::OwnershipMode::Borrowed)
                                )
                                && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                    &sig, sig_param_idx,
                                )
                            {
                                arg_str.trim_start_matches('&').to_string()
                            } else if !arg_str.starts_with('&')
                                && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                    &sig, sig_param_idx,
                                )
                            {
                                apply_borrow(&mut arg_str);
                                arg_str
                            } else {
                                arg_str
                            }
                        }
                        _ => arg_str,
                    };
                    if let Some((_, arg_expr)) = arguments.get(i) {
                        if let Expression::Identifier { name, .. } = arg_expr {
                            if self.identifier_already_ref(name)
                                && arg_str.starts_with('&')
                                && !arg_str.starts_with("&mut ")
                            {
                                arg_str = arg_str[1..].trim_start().to_string();
                            }
                        }
                    }
                    if !arg_str.starts_with('&') {
                        if let (Some(rt), Some((_, arg_expr))) =
                            (receiver_type_name.as_deref(), arguments.get(i))
                        {
                            if let Expression::Identifier { name, .. } = arg_expr {
                                let copy_aggregate_owned_pass = {
                                    let caller_copy = self.current_function_params.iter().any(
                                        |p| {
                                            p.name == *name
                                                && self.is_type_copy(&p.type_)
                                                && !crate::type_classification::is_copy_pass_by_value_formal(
                                                    &p.type_,
                                                )
                                        },
                                    );
                                    caller_copy
                                        && self
                                            .resolve_method_function_signature(
                                                rt,
                                                method,
                                                arguments.len(),
                                            )
                                            .is_some_and(|sig| {
                                                let pidx = sig.arg_param_index(i);
                                                sig.formal_param_type(pidx).is_some_and(|t| {
                                                    let bare = match t {
                                                        Type::Reference(inner)
                                                        | Type::MutableReference(inner) => {
                                                            inner.as_ref()
                                                        }
                                                        other => other,
                                                    };
                                                    self.is_type_copy(bare)
                                                        && !crate::type_classification::is_copy_pass_by_value_formal(
                                                            bare,
                                                        )
                                                })
                                            })
                                };
                                if !copy_aggregate_owned_pass
                                    && !self.emitted_rust_ref_formals.contains(name)
                                    && !self.identifier_already_ref(name)
                                    && !callee_arg_emits_owned
                                    && !self.current_func_is_pure_forwarding_delegate
                                    && self.method_registry_arg_expects_shared_borrow(
                                        rt,
                                        method,
                                        i,
                                        arguments.len(),
                                    )
                                {
                                    crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                        &mut arg_str,
                                    );
                                } else if !self.emitted_rust_ref_formals.contains(name)
                                    && !self.identifier_already_ref(name)
                                    && !self.current_func_is_pure_forwarding_delegate
                                {
                                    if let Some(ms) = self.lookup_method_signature(rt, method) {
                                        let mut reg_sig = ms.to_function_signature();
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
                                        let pidx = reg_sig.arg_param_index(i);
                                        let owned_contract = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                            &reg_sig, pidx,
                                        ) || reg_sig.formal_param_type(pidx).is_some_and(|t| {
                                            let bare = match t {
                                                Type::Reference(inner)
                                                | Type::MutableReference(inner) => inner.as_ref(),
                                                other => other,
                                            };
                                            self.is_type_copy(bare)
                                                && !crate::type_classification::is_copy_pass_by_value_formal(
                                                    bare,
                                                )
                                        });
                                        let caller_copy_aggregate = self
                                            .current_function_params
                                            .iter()
                                            .any(|p| {
                                                p.name == *name
                                                    && self.is_type_copy(&p.type_)
                                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                                        &p.type_,
                                                    )
                                            });
                                        // regression: do not re-borrow Copy aggregates /
                                        // owned-emitted formals from stale Reference(T).
                                        if !owned_contract
                                            && !caller_copy_aggregate
                                            && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                                &reg_sig, pidx,
                                            )
                                        {
                                            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                                &mut arg_str,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // regression: peel stale `&` on Copy-aggregate owned passes (`&through` → `through`).
                    if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
                        if let (Some(rt), Some((_, arg_expr))) =
                            (receiver_type_name.as_deref(), arguments.get(i))
                        {
                            if let Expression::Identifier { name, .. } = arg_expr {
                                let caller_copy = self.current_function_params.iter().any(|p| {
                                    p.name == *name
                                        && self.is_type_copy(&p.type_)
                                        && !crate::type_classification::is_copy_pass_by_value_formal(
                                            &p.type_,
                                        )
                                });
                                let callee_copy = self
                                    .resolve_method_function_signature(
                                        rt,
                                        method,
                                        arguments.len(),
                                    )
                                    .is_some_and(|sig| {
                                        let pidx = sig.arg_param_index(i);
                                        crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                            &sig, pidx,
                                        ) || sig
                                            .formal_param_type(pidx)
                                            .or_else(|| sig.param_types.get(pidx))
                                            .is_some_and(|t| {
                                                let bare = match t {
                                                    Type::Reference(inner)
                                                    | Type::MutableReference(inner) => inner.as_ref(),
                                                    other => other,
                                                };
                                                self.is_type_copy(bare)
                                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                                        bare,
                                                    )
                                            })
                                    });
                                if caller_copy && callee_copy {
                                    arg_str = arg_str.trim_start_matches('&').to_string();
                                }
                            }
                        }
                    }
                    if !arg_str.starts_with('&') && !callee_arg_emits_owned {
                        if let Some((_, arg_expr)) = arguments.get(i) {
                            let arg_binding_already_rust_ref = matches!(
                                arg_expr,
                                Expression::Identifier { name, .. }
                                    if self.identifier_binding_already_rust_ref(name)
                            );
                            let callee_wants_mut = matches!(
                                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                    &sig,
                                    i,
                                    receiver_type_name.as_deref(),
                                ),
                                OwnershipMode::MutBorrowed,
                            ) || sig.param_types.get(sig.arg_param_index(i)).is_some_and(|t| {
                                matches!(t, Type::MutableReference(_))
                            });
                            // Already `&mut T` / `&T` bindings reborrow bare; mut formals
                            // must not get a shared `&` (`&&mut T`).
                            if arg_binding_already_rust_ref || callee_wants_mut {
                                // keep bare / let mut-coercion path handle `&mut`
                            } else if crate::codegen::rust::call_site_borrow::expression_supports_shared_borrow_at_call_site(
                                arg_expr,
                                &arg_str,
                            ) {
                                let pidx = sig.arg_param_index(i);
                                let caller_copy_aggregate = matches!(
                                    arg_expr,
                                    Expression::Identifier { name, .. }
                                        if self.current_function_params.iter().any(|p| {
                                            p.name == *name
                                                && self.is_type_copy(&p.type_)
                                                && !crate::type_classification::is_copy_pass_by_value_formal(
                                                    &p.type_,
                                                )
                                        })
                                );
                                let owned_contract = callee_arg_emits_owned
                                    || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                        &sig, pidx,
                                    )
                                    || sig.formal_param_type(pidx).is_some_and(|t| {
                                        let bare = match t {
                                            Type::Reference(inner)
                                            | Type::MutableReference(inner) => inner.as_ref(),
                                            other => other,
                                        };
                                        self.is_type_copy(bare)
                                            && !crate::type_classification::is_copy_pass_by_value_formal(
                                                bare,
                                            )
                                    });
                                // regression: Copy aggregates / owned emission beat stale Borrowed.
                                if caller_copy_aggregate || owned_contract {
                                    // skip re-borrow
                                } else {
                                let mut needs_borrow = matches!(
                                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_method_arg(
                                        &sig,
                                        i,
                                        receiver_type_name.as_deref(),
                                    ),
                                    OwnershipMode::Borrowed,
                                ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                    &sig, pidx,
                                ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(
                                    &sig, pidx,
                                );
                                if !needs_borrow {
                                    if let Some(rt) = receiver_type_name.as_deref() {
                                        if let Some(global) = self.global_signature_registry() {
                                            use crate::codegen::rust::call_signature_resolution::resolve_method_for_call_site;
                                            if let Some(resolved) =
                                                resolve_method_for_call_site(
                                                    &self.signature_registry,
                                                    Some(global),
                                                    rt,
                                                    method,
                                                    arguments.len(),
                                                )
                                            {
                                                let gsig = resolved.sig;
                                                let gpidx = gsig.arg_param_index(i);
                                                needs_borrow = matches!(
                                                    crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                                                        &gsig, gpidx,
                                                    ),
                                                    OwnershipMode::Borrowed,
                                                ) || crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                                    &gsig, gpidx,
                                                );
                                            }
                                        }
                                    }
                                }
                                if needs_borrow {
                                    apply_borrow(&mut arg_str);
                                }
                                }
                            }
                        }
                    }
                    if callee_arg_emits_owned {
                        if let Some((_, arg_expr)) = arguments.get(i) {
                            if let Expression::Identifier { name, .. } = arg_expr {
                                if self.emitted_rust_ref_formals.contains(name)
                                    && self
                                        .current_function_params
                                        .iter()
                                        .any(|p| p.name == *name)
                                    && !arg_str.ends_with(".clone()")
                                    && !callee_formal_is_copy
                                {
                                    arg_str =
                                        format!("{}.clone()", arg_str.trim_start_matches('&'));
                                }
                            }
                        }
                    }
                    if let Some((_, arg_expr)) = arguments.get(i) {
                        let callee_wants_shared =
                            crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                &sig, sig_param_idx,
                            );
                        let callee_wants_owned = callee_arg_emits_owned;
                        self.finalize_owned_outer_formal_call_arg(
                            &mut arg_str,
                            arg_expr,
                            callee_wants_shared && !callee_wants_owned,
                            callee_wants_owned,
                        );
                        self.maybe_pure_forwarding_strip_call_arg(
                            &mut arg_str,
                            arg_expr,
                            receiver_type_name.as_deref(),
                            Some(method),
                            Some(i),
                            Some(arguments.len()),
                            Some(&sig),
                        );
                        // Single-statement consuming moves: strip over-eager `.clone()`
                        // (`local.merge(remote)`). Finalize runs after arguments.rs.
                        if let Expression::Identifier { name, .. } = arg_expr {
                            if arg_str.ends_with(".clone()")
                                && !(callee_wants_shared && !callee_wants_owned)
                                && self.caller_keeps_owned_outer_formal(name)
                                && self.current_function_body.len() <= 1
                            {
                                crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                    &mut arg_str,
                                );
                            }
                        }
                    }
                    // Copy map/set keys still need `&K`; already-&str / shared-ref bindings must not.
                    // Delegate to the shared helper (same path as arguments.rs / trait calls).
                    if let Some((_, arg_expr)) = arguments.get(i) {
                        let arg_already_rust_ref = match arg_expr {
                            Expression::Identifier { name, .. } => {
                                self.identifier_already_ref(name)
                                    || self.str_ref_optimized_params.contains(name.as_str())
                                    || self.emitted_rust_ref_formals.contains(name.as_str())
                            }
                            Expression::Unary {
                                op: UnaryOp::Ref, ..
                            } => true,
                            _ => false,
                        };
                        let arg_binding_already_shared_ref =
                            matches!(arg_expr, Expression::Identifier { name, .. }
                                if self.identifier_already_ref(name)
                                    || self.str_ref_optimized_params.contains(name.as_str())
                                    || self.emitted_rust_ref_formals.contains(name.as_str())
                                    || self.inferred_borrowed_params.contains(name));
                        crate::codegen::rust::call_site_borrow::finalize_collection_key_call_site_arg(
                            Some(sig),
                            i,
                            arg_expr,
                            &mut arg_str,
                            arg_already_rust_ref,
                            receiver_type_name.as_deref(),
                            arg_binding_already_shared_ref,
                        );
                    }
                    // Final strip: already-emitted shared-ref formals must not become `&&T`.
                    if let Some((_, Expression::Identifier { name, .. })) = arguments.get(i) {
                        if (self.emitted_rust_ref_formals.contains(name.as_str())
                            || self.str_ref_optimized_params.contains(name.as_str())
                            || self.identifier_already_ref(name)
                            || (self.inferred_borrowed_params.contains(name.as_str())
                                && !self.collection_key_owned_params.contains(name.as_str())
                                && self.current_function_params.iter().any(|p| {
                                    p.name == *name
                                        && crate::codegen::rust::types::is_windjammer_text_type(
                                            &p.type_,
                                        )
                                })))
                            && !self.collection_key_owned_params.contains(name.as_str())
                            && arg_str.starts_with('&')
                            && !arg_str.starts_with("&mut ")
                        {
                            let base =
                                crate::codegen::rust::expression_utilities::borrow_base_expr(
                                    &arg_str,
                                );
                            if base == name.as_str() {
                                arg_str = name.clone();
                            }
                        }
                    }
                    arg_str
                })
                .collect()
        } else {
            args
        };

        // E0499 FIX: Extract temporaries when receiver and arguments both borrow self.
        // Pattern: self.field.method(self.other_method()) generates two &mut self borrows.
        // Fix: { let __wj_tmp0 = self.other_method(); self.field.method(__wj_tmp0) }
        // Split borrows on different fields (self.dash.activate(&mut self.player)) need no temp.
        let receiver_borrows_self = self.codegen_expression_traces_to_self(object);
        let mut self_borrow_temps: Vec<(String, String)> = Vec::new();
        let args = if receiver_borrows_self {
            let needs_extraction = arguments.iter().any(|(_label, arg)| {
                self.expression_borrows_self(arg) && !self.disjoint_self_field_accesses(object, arg)
            });
            if needs_extraction {
                args.into_iter()
                    .enumerate()
                    .map(|(i, mut arg_str)| {
                        let (_label, arg_expr) = &arguments[i];
                        if self.expression_borrows_self(arg_expr)
                            && !self.disjoint_self_field_accesses(object, arg_expr)
                        {
                            let temp_name = format!("__wj_tmp{}", i);
                            self_borrow_temps.push((temp_name.clone(), arg_str));
                            temp_name
                        } else if self.disjoint_self_field_accesses(object, arg_expr) {
                            if let Some(ref sig) = resolved_signature {
                                let sig_param_idx = sig.arg_param_index(i);
                                let param_is_mut_borrowed = sig
                                    .param_ownership
                                    .get(sig_param_idx)
                                    .is_some_and(|o| matches!(o, OwnershipMode::MutBorrowed))
                                    || sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                        matches!(t, Type::MutableReference(_))
                                    });
                                if param_is_mut_borrowed {
                                    crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                                        arg_expr,
                                        &mut arg_str,
                                        &self.current_function_params,
                                        &self.inferred_mut_borrowed_params,
                                    );
                                }
                            }
                            arg_str
                        } else {
                            arg_str
                        }
                    })
                    .collect()
            } else {
                args.into_iter()
                    .enumerate()
                    .map(|(i, mut arg_str)| {
                        let (_label, arg_expr) = &arguments[i];
                        if self.disjoint_self_field_accesses(object, arg_expr) {
                            if let Some(ref sig) = resolved_signature {
                                let sig_param_idx = sig.arg_param_index(i);
                                let param_is_mut_borrowed = sig
                                    .param_ownership
                                    .get(sig_param_idx)
                                    .is_some_and(|o| matches!(o, OwnershipMode::MutBorrowed))
                                    || sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                        matches!(t, Type::MutableReference(_))
                                    });
                                if param_is_mut_borrowed {
                                    crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                                        arg_expr,
                                        &mut arg_str,
                                        &self.current_function_params,
                                        &self.inferred_mut_borrowed_params,
                                    );
                                }
                            }
                        }
                        arg_str
                    })
                    .collect()
            }
        } else {
            args
        };

        // Restore float target type after argument generation
        self.assignment_float_target_type = prev_float_target;

        // Generate turbofish if present, or infer for collect() from return type
        let mut collect_adapter = String::new();
        let turbofish = if let Some(types) = type_args {
            let type_strs: Vec<String> = types.iter().map(|t| self.type_to_rust(t)).collect();
            format!("::<{}>", type_strs.join(", "))
        } else if method == "collect" {
            let (adapter, turbo) = self.compute_collect_lowering(object);
            collect_adapter = adapter;
            turbo
        } else {
            String::new()
        };
        if !collect_adapter.is_empty() {
            obj_str.push_str(&collect_adapter);
        }

        // Special case: empty method name means turbofish on a function call (func::<T>())
        if method.is_empty() {
            return format!("{}{}({})", obj_str, turbofish, args.join(", "));
        }

        // Special case: substring(start, end) -> text[start..end] (or owned String in match arms)
        if method == "substring" && args.len() == 2 {
            let mut start_str = crate::codegen::rust::expression_utilities::strip_shared_borrow_prefix(
                &args[0],
            );
            let mut end_str = crate::codegen::rust::expression_utilities::strip_shared_borrow_prefix(
                &args[1],
            );
            if let Some((_, start_expr)) = arguments.first() {
                self.maybe_cast_index_to_usize(&mut start_str, start_expr);
            }
            if let Some((_, end_expr)) = arguments.get(1) {
                self.maybe_cast_index_to_usize(&mut end_str, end_expr);
            }
            let slice_inner = format!("{}[{}..{}]", obj_str, start_str, end_str);
            let needs_owned = self.in_match_arm_needing_string
                || self.coerce_string_literals_to_owned
                || self.in_owned_value_context
                || self.in_call_argument_generation
                || (self.in_expression_context
                    && crate::codegen::rust::string_utilities::return_type_expects_owned_string(
                        &self.current_function_return_type,
                    ));
            if needs_owned {
                return format!("({slice_inner}).to_string()");
            }
            // Chained receiver (`substring(..).trim()`): emit bare slice so precedence
            // yields `(text[s..e]).trim()`, not `&(text[s..e].trim())`.
            if self.in_field_access_object {
                return slice_inner;
            }
            // Standalone / `let` binding: bare `[s..e]` is unsized — emit `&str`.
            return format!("&{slice_inner}");
        }

        // Signature-driven: owned String-producing args at `&str` / Pattern formals need
        // a borrow (or bare literal). Prefer `&expr.to_string()` so source `.to_string()`
        // remains visible (TDD). Text receivers include `&str` formals — registry keys are
        // still `String::method` in stdlib_meta.
        let receiver_is_text = receiver_type_name.as_deref().is_some_and(|rt| {
            rt == "String"
                || rt == "string"
                || rt == "str"
                || rt.ends_with("::String")
                || rt.ends_with("::str")
        }) || matches!(
            object,
            Expression::Identifier { name, .. }
                if self.local_var_types.get(name).is_some_and(|t| {
                    crate::codegen::rust::types::is_windjammer_text_type(t)
                }) || self.str_ref_optimized_params.contains(name)
                    || self.emitted_rust_ref_formals.contains(name)
                    || (self.inferred_borrowed_params.contains(name.as_str())
                        && self.current_function_params.iter().any(|p| {
                            p.name == *name
                                && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        }))
                    || self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    })
        ) || self
            .infer_expression_type(object)
            .as_ref()
            .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);

        // Determine separator: :: for static/module calls, . for instance methods
        // - Type/Module (starts with uppercase): use ::
        // - Variable (starts with lowercase): use .
        let separator = match object {
            Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
            Expression::Identifier { name, .. } => {
                // Enum variant paths parse as one identifier: `ShaderFile::HiZCull.to_path()`
                if Self::is_enum_variant_qualified_path(name) {
                    "."
                } else if self.identifier_is_static_call_root(name) {
                    "::" // Vec::new(), std::fs::read(), tokio::spawn()
                } else {
                    "." // x.abs(), value.method()
                }
            }
            Expression::FieldAccess { ref object, .. } => {
                // Type::Variant.method() in Windjammer (enum variant receiver) must lower to
                // `(Type::Variant).method()` in Rust — not `Type::Variant::method()`.
                match object {
                    Expression::Identifier { name, .. }
                        if name.chars().next().is_some_and(|c| c.is_uppercase()) =>
                    {
                        "." // ShaderFile::HiZCull.to_path() → (ShaderFile::HiZCull).to_path()
                    }
                    Expression::Identifier { name, .. }
                        if self.identifier_is_static_call_root(name) =>
                    {
                        "::" // std::fs::read(), module paths
                    }
                    _ => ".", // self.field.method()
                }
            }
            _ => ".", // Instance method on expressions
        };

        // SPECIAL CASE: .slice() method is our desugared slice syntax [start..end]
        // Convert it back to proper Rust slice syntax
        // For strings, we need to add & to get &str (a reference)
        if method == "slice" && args.len() == 2 {
            return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
        }

        // Explicit `.clone()` in WJ source is Rust leakage (W0005). Prefer stripping, but
        // preserve when reuse analysis, loop body, or call-argument context needs it
        // (WDB-105 loop / WDB-106 sequential owned moves — stripping leaves E0382).
        if crate::type_classification::is_language_level_explicit_clone(method)
            && arguments.is_empty()
        {
            let preserve_in_loop = self.loop_body_depth > 0;
            let preserve_for_reuse = matches!(object, Expression::Identifier { name, .. } if {
                self.auto_clone_analysis.as_ref().is_some_and(|a| {
                    a.needs_clone(name, self.current_statement_idx).is_some()
                        || a.needs_clone_anywhere(name)
                })
            });
            // Call-arg sites: analysis often misses reuse because the explicit clone
            // itself masks the first move; keep the user's clone (WDB-106).
            let preserve_call_arg = self.in_call_argument_generation
                && matches!(object, Expression::Identifier { .. });
            if preserve_in_loop || preserve_for_reuse || preserve_call_arg {
                if let Expression::Identifier { name, .. } = object {
                    let is_borrowed_string = self.inferred_borrowed_params.contains(name)
                        && self.current_function_params.iter().any(|p| {
                            p.name == *name
                                && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        });
                    if is_borrowed_string {
                        return format!("{}.to_string()", obj_str);
                    }
                }
                return format!("{}.clone()", obj_str.trim_end_matches(".clone()"));
            }
            return crate::codegen::rust::string_utilities::lower_explicit_clone_call(
                object,
                &obj_str,
                &self.inferred_borrowed_params,
                &self.current_function_params,
            );
        }

        // UI FRAMEWORK: Check if we need to add .to_vnode() for .child() methods
        // DISABLED: Too aggressive - needs type checking to determine if parameter expects VNode
        // TODO: Re-enable with proper type checking when VNode type bindings are implemented
        let mut processed_args = args;
        // Always strip stale `&` on already-emitted shared-ref formals (`key: &str` →
        // `map.get(key)`, never `map.get(&key)` / `&&str`).
        for (i, arg_str) in processed_args.iter_mut().enumerate() {
            let Some((_, arg_expr)) = arguments.get(i) else {
                continue;
            };
            self.strip_stale_amp_on_already_ref_arg(arg_expr, arg_str);
        }
        for (i, arg_str) in processed_args.iter_mut().enumerate() {
            let Some((_, arg_expr)) = arguments.get(i) else {
                continue;
            };
            let expects_pattern =
                crate::codegen::rust::string_utilities::method_call_arg_expects_pattern_str(
                    method,
                    i,
                    resolved_signature.as_ref().or(method_signature.as_ref()),
                    receiver_type_name.as_deref(),
                    receiver_is_text,
                    &self.signature_registry,
                );
            let expects_collection_key =
                crate::codegen::rust::stdlib_method_traits::method_is_map_key_qualified(
                    method,
                    receiver_type_name.as_deref(),
                    &self.signature_registry,
                ) || resolved_signature
                    .as_ref()
                    .or(method_signature.as_ref())
                    .is_some_and(|sig| {
                        crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup(
                            sig,
                            i,
                            receiver_type_name.as_deref(),
                        )
                    });
            if std::env::var("WJ_DEBUG_FIND_PATTERN").is_ok() && method == "find" {}
            if expects_pattern || expects_collection_key {
                crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                    arg_expr,
                    arg_str,
                );
                if std::env::var("WJ_DEBUG_FIND_PATTERN").is_ok() && method == "find" {}
            }
        }
        if let Some(ref rt) = receiver_type_name {
            let qualified = format!("{rt}::{method}");
            for (i, arg_str) in processed_args.iter_mut().enumerate() {
                if arg_str.starts_with('&') {
                    continue;
                }
                let Some((_, arg_expr)) = arguments.get(i) else {
                    continue;
                };
                let Expression::Identifier { name, .. } = arg_expr else {
                    continue;
                };
                let wants_borrow = self
                    .get_signature_with_global(&qualified)
                    .map(|sig| {
                        let pidx = sig.arg_param_index(i);
                        crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            sig, pidx,
                        ) || sig
                            .param_types
                            .get(pidx)
                            .is_some_and(|t| matches!(t, Type::Reference(_)))
                    })
                    .unwrap_or(false)
                    || self.method_registry_arg_expects_shared_borrow(
                        rt,
                        method,
                        i,
                        arguments.len(),
                    );
                if !wants_borrow {
                    continue;
                }
                // Already-emitted shared-ref bindings are already borrowed.
                if self.emitted_rust_ref_formals.contains(name.as_str())
                    || self.str_ref_optimized_params.contains(name.as_str())
                    || self.identifier_already_ref(name)
                {
                    continue;
                }
                let is_owned_collection_local =
                    self.local_var_types.get(name).is_some_and(|t| {
                        crate::type_classification::type_is_vec_container(t)
                            || matches!(
                                t,
                                Type::Parameterized(n, _)
                                    if crate::type_classification::is_stdlib_collection_type_name(n)
                            )
                    }) || self.infer_expression_type(arg_expr).is_some_and(|t| {
                        crate::type_classification::type_is_vec_container(&t)
                            || matches!(
                                &t,
                                Type::Parameterized(n, _)
                                    if crate::type_classification::is_stdlib_collection_type_name(n)
                            )
                    });
                if is_owned_collection_local {
                    *arg_str = format!("&{arg_str}");
                }
            }
        }

        // Consuming iterable adapters on a borrowed receiver (`&Vec` / `&mut self`
        // field) yield references; rewrite to `.iter().copied()/cloned()` so
        // downstream adapters see owned items. Signature-driven via
        // `method_returns_iterable_qualified` + owned-self meta (covers `into_iter`).
        if arguments.is_empty()
            && crate::codegen::rust::stdlib_method_traits::method_returns_iterable_qualified(
                method,
                receiver_type_name.as_deref(),
                &self.signature_registry,
            )
        {
            let consumes_receiver = resolved_signature
                .as_ref()
                .or(method_signature.as_ref())
                .is_some_and(|sig| {
                    sig.has_self_receiver
                        && matches!(
                            sig.param_ownership.first(),
                            Some(crate::analyzer::OwnershipMode::Owned)
                        )
                })
                || crate::analyzer::stdlib_method_traits::method_call_consumes_receiver(
                    method,
                    receiver_type_name.as_deref(),
                    &self.signature_registry,
                );
            if consumes_receiver {
                if let Some(elem_ty) = self.infer_borrowed_collection_element_type(object) {
                    let prev_field_access = self.in_field_access_object;
                    self.in_field_access_object = true;
                    let mut recv_str = self.generate_expression(object);
                    self.in_field_access_object = prev_field_access;
                    if recv_str.starts_with("&mut ") {
                        recv_str = recv_str["&mut ".len()..].to_string();
                    } else if recv_str.starts_with('&') {
                        recv_str = recv_str[1..].to_string();
                    }
                    if self.is_type_copy(&elem_ty) {
                        return format!("{recv_str}.iter().copied()");
                    }
                    return format!("{recv_str}.iter().cloned()");
                }
            }
        }

        // reversed() → into_iter().rev().collect::<Vec<_>>()
        if method == "reversed" && processed_args.is_empty() {
            return format!("{}.into_iter().rev().collect::<Vec<_>>()", obj_str);
        }
        // enumerate() → iter().enumerate()
        // Rust Vec doesn't have .enumerate() — only iterators do.
        // But if the object already ends with .iter(), .iter_mut(), or
        // .into_iter(), don't add a redundant .iter() prefix.
        if method == "enumerate" && processed_args.is_empty() {
            let already_iterator = obj_str.ends_with(".iter()")
                || obj_str.ends_with(".iter_mut()")
                || obj_str.ends_with(".into_iter()");
            if already_iterator {
                return format!("{}.enumerate()", obj_str);
            } else {
                return format!("{}.iter().enumerate()", obj_str);
            }
        }

        // TDD FIX (Bug #3): Extract format!() / write!-block macros in method arguments too
        let needs_format_temp = |arg_str: &str| -> bool {
            arg_str.contains("format!(") || arg_str.contains("write!(&mut __s,")
        };
        let has_format_arg = processed_args
            .iter()
            .any(|arg_str| needs_format_temp(arg_str));

        let base_expr = if has_format_arg {
            // Extract format!() macros to temp variables
            let mut temp_decls = String::new();
            let mut temp_counter = 0i32;
            let fixed_args: Vec<String> = processed_args
                .iter()
                .enumerate()
                .map(|(arg_idx, arg_str)| {
                    let has_borrow_prefix = arg_str.starts_with('&');
                    let inner = if has_borrow_prefix {
                        &arg_str[1..]
                    } else {
                        arg_str.as_str()
                    };
                    let needs_extract = inner.starts_with("format!(")
                        || (inner.starts_with('{') && inner.contains("write!(&mut __s,"));
                    if needs_extract {
                        let temp_name = format!("_temp{}", temp_counter);
                        temp_counter += 1;
                        temp_decls.push_str(&format!("let {} = {}; ", temp_name, inner));

                        let sig_for_format = resolved_signature
                            .as_ref()
                            .or(method_signature.as_ref());
                        // Global registry may mark owned String when local sig is incomplete.
                        if !has_borrow_prefix
                            && self.mc_method_param_expects_owned_string_from_global(
                                object,
                                method,
                                arg_idx,
                                arguments.len(),
                            )
                            && !sig_for_format.is_some_and(|sig| {
                                let pi = sig.arg_param_index(arg_idx);
                                crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                                    sig, pi,
                                )
                            })
                        {
                            temp_name
                        } else {
                            crate::codegen::rust::call_site_borrow::format_temp_arg_pass_expr(
                                sig_for_format,
                                arg_idx,
                                &temp_name,
                                has_borrow_prefix,
                            )
                        }
                    } else {
                        arg_str.clone()
                    }
                })
                .collect();

            // Wrap in block: { let _temp0 = format!(...); obj.method(&_temp0, ...) }
            format!(
                "{{ {}{}{}{}{}({}) }}",
                temp_decls,
                obj_str,
                separator,
                method,
                turbofish,
                fixed_args.join(", ")
            )
        } else {
            let emit_method = if separator == "::" {
                let qualified = format!("{obj_str}::{method}");
                crate::analyzer::SignatureRegistry::resolve_runtime_emit_method_name_chain(
                    &qualified,
                    &self.signature_registry,
                    self.global_signature_registry.as_deref(),
                )
                .unwrap_or_else(|| method.to_string())
            } else {
                method.to_string()
            };
            format!(
                "{}{}{}{}({})",
                obj_str,
                separator,
                emit_method,
                turbofish,
                processed_args.join(", ")
            )
        };

        // E0499 FIX: Wrap in block with temporaries if self-borrow extraction was needed
        let base_expr = if !self_borrow_temps.is_empty() {
            let mut temp_decls = String::new();
            for (name, value) in &self_borrow_temps {
                temp_decls.push_str(&format!("let {} = {}; ", name, value));
            }
            format!("{{ {}{} }}", temp_decls, base_expr)
        } else {
            base_expr
        };

        // Iterator adapters yielding `Option<&T>` when the function needs `Option<T>`.
        // Gated by iterator-chain detection + type equivalence (not method-name lists).
        if self.find_needs_cloned_for_owned_return(object) && !base_expr.ends_with(".cloned()") {
            return format!("{}.cloned()", base_expr);
        }

        base_expr
    }

    /// `Type::Variant` in expressions is parsed as a single qualified identifier, not FieldAccess.
    pub(in crate::codegen::rust) fn is_enum_variant_qualified_path(name: &str) -> bool {
        crate::type_classification::is_enum_variant_constructor_path(name)
            && name.matches("::").count() == 1
    }
}
