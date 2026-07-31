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
        let args = if let Some(ref sig) = resolved_signature {
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
                                None,
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
                            if is_collection_key || !can_mut {
                                apply_borrow(&mut arg_str);
                                arg_str
                            } else if callee_formal_is_copy
                                && callee_arg_emits_owned
                                && !matches!(ownership, OwnershipMode::MutBorrowed)
                            {
                                // Owned Copy aggregates (`mut deps: AppDeps`) pass by value.
                                // Cross-file `function_emitted_mut_arg_indices` may upgrade
                                // local `ownership` to MutBorrowed while the looked-up sig still
                                // claims an owned Copy contract — honor the upgrade (`&mut
                                // self.player` into `player: &mut PlayerState`).
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
                                        // WDB-060: do not re-borrow Copy aggregates /
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
                    // WDB-060: peel stale `&` on Copy-aggregate owned passes (`&through` → `through`).
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
                            if crate::codegen::rust::call_site_borrow::expression_supports_shared_borrow_at_call_site(
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
                                // WDB-060: Copy aggregates / owned emission beat stale Borrowed.
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
        let turbofish = if let Some(types) = type_args {
            let type_strs: Vec<String> = types.iter().map(|t| self.type_to_rust(t)).collect();
            format!("::<{}>", type_strs.join(", "))
        } else if method == "collect" {
            if let Some(target_ty) = &self.collect_target_type {
                format!("::<{}>", self.type_to_rust(target_ty))
            } else if let Some(ret_ty) = &self.current_function_return_type {
                format!("::<{}>", self.type_to_rust(ret_ty))
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Special case: empty method name means turbofish on a function call (func::<T>())
        if method.is_empty() {
            return format!("{}{}({})", obj_str, turbofish, args.join(", "));
        }

        // Special case: substring(start, end) -> &text[start..end]
        if method == "substring" && args.len() == 2 {
            return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
        }

        // Signature-driven: owned String-producing args at `&str` formals need a borrow.
        // Prefer `&expr.to_string()` so source `.to_string()` remains visible (TDD).
        if args.len() == 1 {
            let receiver_is_string = receiver_type_name.as_deref().is_some_and(|rt| {
                rt == "String" || rt == "string" || rt.ends_with("::String")
            }) || matches!(
                object,
                Expression::Identifier { name, .. }
                    if self.local_var_types.get(name).is_some_and(|t| {
                        matches!(t, Type::String)
                            || matches!(t, Type::Custom(n) if n == "String" || n == "string")
                    })
            );
            let param_wants_str_ref = resolved_signature
                .as_ref()
                .and_then(|sig| sig.param_type_for_arg(0))
                .is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                })
                || receiver_type_name.as_deref().is_some_and(|rt| {
                    self.lookup_method_signature(rt, method)
                        .and_then(|ms| ms.param_types.first())
                        .is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        })
                })
                || (receiver_is_string
                    && self
                        .lookup_method_signature("String", method)
                        .and_then(|ms| ms.param_types.first())
                        .is_some_and(|t| {
                            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        }));
            if param_wants_str_ref {
                if let Some((_label, arg)) = arguments.first() {
                    // `"lit".to_string()` at a &str param is already satisfied by the literal.
                    if let Expression::MethodCall {
                        object,
                        method: m,
                        ..
                    } = arg
                    {
                        if m == "to_string" {
                            if let Expression::Literal {
                                value: Literal::String(_),
                                ..
                            } = **object
                            {
                                let lit = args[0].trim_start_matches('&');
                                let bare = lit.strip_suffix(".to_string()").unwrap_or(lit);
                                return format!("{}.{}({})", obj_str, method, bare);
                            }
                            // Prefer `&expr.to_string()` (coerces to &str); keeps `.to_string()`
                            // visible for non-string→String conversions (TDD: to_string_push_str).
                            if !args[0].starts_with('&') {
                                return format!("{}.{}(&{})", obj_str, method, args[0]);
                            }
                        }
                    }
                    // Generated arg may already be `….to_string()` even if AST shape differs.
                    if args[0].ends_with(".to_string()") && !args[0].starts_with('&') {
                        return format!("{}.{}(&{})", obj_str, method, args[0]);
                    }
                }
            }
        }

        // Determine separator: :: for static/module calls, . for instance methods
        // - Type/Module (starts with uppercase): use ::
        // - Variable (starts with lowercase): use .
        let separator = match object {
            Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
            Expression::Identifier { name, .. } => {
                // Enum variant paths parse as one identifier: `ShaderFile::HiZCull.to_path()`
                if Self::is_enum_variant_qualified_path(name) {
                    "."
                } else {
                    // Check for known module/crate names that should use ::
                    // Note: Avoid common variable names like "path", "config" which are used as variables
                    // Only unambiguous module/type names — never short names used as variables (io, log, fs, …).
                    let known_modules = [
                        "std",
                        "serde_json",
                        "serde",
                        "tokio",
                        "reqwest",
                        "sqlx",
                        "chrono",
                        "sha2",
                        "bcrypt",
                        "base64",
                        "rand",
                        "Vec",
                        "String",
                        "Option",
                        "Result",
                        "Box",
                        "Arc",
                        "Mutex",
                        "Utc",
                        "Local",
                        "DEFAULT_COST",
                    ];

                    // Type, `Self`, or module (uppercase) vs variable (lowercase)
                    if name == "Self"
                        || name.chars().next().is_some_and(|c| c.is_uppercase())
                        || name.contains('.')
                        || known_modules.contains(&name.as_str())
                        || self.is_imported_runtime_std_module(name)
                    {
                        "::" // Vec::new(), std::fs::read(), serde_json::to_string()
                    } else {
                        "." // x.abs(), value.method()
                    }
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
                    Expression::Identifier { name, .. } if name == "std" => "::",
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

        // E0308: Borrowed Windjammer `string` parameters lower to `&str`. `.clone()` on `&str`
        // is still `&str`, but users mean an owned copy → emit `.to_string()`.
        if method == "clone" && arguments.is_empty() {
            if let Expression::Identifier { name, .. } = object {
                if self.inferred_borrowed_params.contains(name.as_str())
                    && self
                        .current_function_params
                        .iter()
                        .find(|p| p.name == *name)
                        .is_some_and(|p| {
                            crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        })
                {
                    return format!("{}.to_string()", obj_str);
                }
            }
        }

        // PHASE 2 OPTIMIZATION: Eliminate unnecessary .clone() calls
        // DISABLED: This optimization was too aggressive and removed needed clones
        // TODO: Make this more conservative - only remove clone when we can prove
        // the value is Copy or when it's the last use
        // if method == "clone" && arguments.is_empty() {
        //     if let Expression::Identifier { name: ref var_name, location: None } = **object {
        //         if self.clone_optimizations.contains(var_name) {
        //             // Skip the .clone(), just return the variable (or borrow if needed)
        //             return obj_str;
        //         }
        //     }
        // }

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
                        ) || sig.param_types.get(pidx).is_some_and(|t| {
                            matches!(t, Type::Reference(_))
                        })
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
                let is_vec_local = self.local_var_types.get(name).is_some_and(|t| {
                    matches!(t, Type::Vec(_))
                        || matches!(t, Type::Parameterized(n, _) if n == "Vec")
                }) || self.infer_expression_type(arg_expr).is_some_and(|t| {
                    matches!(t, Type::Vec(_))
                        || matches!(t, Type::Parameterized(n, _) if n == "Vec")
                });
                if is_vec_local {
                    *arg_str = format!("&{arg_str}");
                }
            }
        }

        // WINDJAMMER STDLIB → RUST TRANSLATION
        // Some Windjammer methods don't exist in Rust and need translation.
        //
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
            format!(
                "{}{}{}{}({})",
                obj_str,
                separator,
                method,
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

        base_expr
    }

    /// `Type::Variant` in expressions is parsed as a single qualified identifier, not FieldAccess.
    pub(in crate::codegen::rust) fn is_enum_variant_qualified_path(name: &str) -> bool {
        let mut parts = name.split("::");
        let type_name = parts.next();
        let variant = parts.next();
        parts.next().is_none()
            && type_name.is_some_and(|t| t.chars().next().is_some_and(|c| c.is_uppercase()))
            && variant.is_some_and(|v| {
                !v.is_empty()
                    && !v.starts_with('<')
                    && v.chars().all(|c| c.is_alphanumeric() || c == '_')
            })
    }
}
