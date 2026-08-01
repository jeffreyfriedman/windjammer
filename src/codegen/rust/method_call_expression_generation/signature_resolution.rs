//! Method call signature resolution — delegates to unified resolver.

use crate::analyzer::FunctionSignature;
use crate::parser::Expression;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Resolve the receiver type for method signature lookup (`self.field` → field type, not owner struct).
    pub(in crate::codegen::rust) fn mc_infer_method_receiver_type_name(
        &self,
        object: &Expression<'ast>,
    ) -> Option<String> {
        if let Expression::Identifier { name, .. } = object {
            if (name == "self" || name == "Self") && self.in_impl_block {
                return self.current_struct_name.clone();
            }
        }
        if let Expression::FieldAccess {
            object: obj, field, ..
        } = object
        {
            if let Expression::Identifier { name, .. } = &**obj {
                if name == "self" {
                    if let Some(sn) = &self.current_struct_name {
                        if let Some(fields) = self.lookup_struct_field_types(sn) {
                            if let Some(ft) = fields.get(field.as_str()) {
                                if let Some(tn) = Self::type_to_name(ft) {
                                    return Some(tn);
                                }
                            }
                        }
                    }
                }
            }
            if let Some(field_type) = self.infer_expression_type(object) {
                if let Some(name) = Self::type_to_name(&field_type) {
                    return Some(name);
                }
            }
        }
        self.infer_type_name(object)
    }

    pub(in crate::codegen::rust) fn mc_resolve_method_call_signature(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> Option<FunctionSignature> {
        use crate::codegen::rust::call_signature_resolution::finalize_call_site_signature;

        let type_name = self
            .mc_infer_method_receiver_type_name(object)
            .or_else(|| self.infer_type_name(object))
            .or_else(|| {
                if let Expression::Identifier { name, .. } = object {
                    if (name == "Self" || name == "self") && self.in_impl_block {
                        return self.current_struct_name.clone();
                    }
                }
                None
            });

        if let Some(ref tn) = type_name {
            use crate::codegen::rust::call_signature_resolution::{
                finalize_call_site_signature, resolve_method_for_call_site, validate_arg_count,
                ResolutionMethod, ResolvedSignature,
            };

            let inferred_receiver_ty = self.infer_expression_type(object);
            let from_method_registry = self.lookup_method_signature(tn, method).and_then(|ms| {
                let mut sig = ms.to_function_signature();
                let qualified = format!("{tn}::{method}");
                if let Some(reg) = self.get_signature_with_global(&qualified) {
                    if reg.emitted_rust_ref_params.is_some() {
                        crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                            &mut sig, reg,
                        );
                    }
                }
                if let Some(recv_ty) = crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                    Some(tn),
                    inferred_receiver_ty.as_ref(),
                    self.current_function_return_type.as_ref(),
                ) {
                    crate::codegen::rust::stdlib_signature_specialization::specialize_signature_for_receiver(
                        &mut sig, &recv_ty,
                    );
                }
                if validate_arg_count(&sig, arguments.len()) {
                    Some(ResolvedSignature {
                        sig,
                        qualified_key: qualified,
                        resolution_method: ResolutionMethod::MethodRegistry,
                        has_collision: false,
                    })
                } else {
                    None
                }
            });

            let from_registry = resolve_method_for_call_site(
                &self.signature_registry,
                self.global_signature_registry(),
                tn,
                method,
                arguments.len(),
            );

            // Unified registry resolution (local + global, emitted-formal refresh) wins over
            // bare MethodSignature stubs that drop `emitted_rust_ref_params`.
            if let Some(resolved) = from_registry {
                let mut sig = resolved.sig;
                if let Some(global) = self.global_signature_registry() {
                    crate::codegen::rust::call_signature_resolution::apply_trait_owned_string_call_site_contracts(
                        global,
                        method,
                        &mut sig,
                    );
                }
                return Some(finalize_call_site_signature(sig));
            }

            if let Some(resolved) = from_method_registry {
                let mut sig = resolved.sig;
                if let Some(global) = self.global_signature_registry() {
                    crate::codegen::rust::call_signature_resolution::apply_trait_owned_string_call_site_contracts(
                        global,
                        method,
                        &mut sig,
                    );
                }
                return Some(finalize_call_site_signature(sig));
            }

            return self
                .lookup_method_signature_on_receiver_type(tn, method, arguments.len())
                .map(finalize_call_site_signature);
        }

        // Never homonym-guess `Type::method` when the receiver is a field access — e.g.
        // `self.quest_manager.is_quest_active` must not resolve to `DialogueState::is_quest_active`.
        if matches!(object, Expression::FieldAccess { .. }) {
            return None;
        }

        // No receiver type known: suffix-match with arg-count validation.
        // Never do bare `get_signature(method)` — it could pick any type's method.
        // Skip `remove` specifically because it has incompatible semantics across types:
        // Vec::remove(usize) takes owned index, HashMap::remove(&K) takes borrowed key.
        if method == "remove" {
            return None;
        }
        let local_sig = self
            .signature_registry
            .find_signature_by_name_and_arg_count(method, arguments.len())
            .cloned();
        let global_sig = self
            .global_signature_registry()
            .and_then(|g| g.find_signature_by_name_and_arg_count(method, arguments.len()))
            .cloned();
        match (local_sig, global_sig) {
            (Some(l), Some(g)) => {
                use crate::codegen::rust::call_signature_resolution::ResolutionMethod;
                use crate::codegen::rust::call_signature_resolution::ResolvedSignature;
                let lr = ResolvedSignature {
                    qualified_key: l.name.clone(),
                    has_collision: false,
                    resolution_method: ResolutionMethod::ReceiverQualified,
                    sig: l,
                };
                let gr = ResolvedSignature {
                    qualified_key: g.name.clone(),
                    has_collision: false,
                    resolution_method: ResolutionMethod::ReceiverQualified,
                    sig: g,
                };
                crate::codegen::rust::call_signature_resolution::pick_best_resolved_signature(
                    Some(lr),
                    Some(gr),
                )
                .map(|r| finalize_call_site_signature(r.sig))
            }
            (Some(l), None) => Some(finalize_call_site_signature(l)),
            (None, Some(g)) => Some(finalize_call_site_signature(g)),
            (None, None) => None,
        }
    }

    /// Single source of truth for call-site signature selection.
    ///
    /// `mc_resolve_method_call_signature` already runs `pick_best_resolved_signature`
    /// (method registry vs global). Downstream must not re-resolve via global first — that
    /// resurrects stale declaration stubs with bare `Vec` + `Owned` metadata.
    pub(in crate::codegen::rust) fn mc_select_call_site_signature(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        resolved_from_mc: &Option<FunctionSignature>,
    ) -> Option<FunctionSignature> {
        use crate::codegen::rust::call_signature_resolution::{
            finalize_call_site_signature, has_stale_owned_non_copy_params, validate_arg_count,
        };
        use crate::codegen::rust::signature_promotion::{
            converged_has_reference_params_over_bare, prefer_converged_over_stub,
        };

        let is_usable = |sig: &FunctionSignature| {
            validate_arg_count(sig, arguments.len()) && !has_stale_owned_non_copy_params(sig)
        };

        let trace = std::env::var("WJ_SIGNATURE_TRACE").is_ok();

        let receiver_type_name = self
            .mc_infer_method_receiver_type_name(object)
            .or_else(|| self.infer_type_name(object))
            .or_else(|| {
                if let Expression::Identifier { name, .. } = object {
                    if (name == "Self" || name == "self") && self.in_impl_block {
                        return self.current_struct_name.clone();
                    }
                }
                None
            });

        // Re-resolve against the merged global registry (may differ from the first
        // mc_resolve snapshot when per-file stubs gained Borrowed ownership but kept bare
        // param_types during Step 4B-a).
        let mc_resolved = self
            .mc_resolve_method_call_signature(object, method, arguments)
            .filter(|s| is_usable(s));

        let global_upgraded = receiver_type_name.as_ref().and_then(|tn| {
            use crate::codegen::rust::call_signature_resolution::resolve_method_for_call_site;
            resolve_method_for_call_site(
                &self.signature_registry,
                self.global_signature_registry(),
                tn,
                method,
                arguments.len(),
            )
            .map(|r| finalize_call_site_signature(r.sig))
            .filter(|g| is_usable(g))
        });

        let prefer_global_over =
            |local: &FunctionSignature, better: &FunctionSignature| -> bool {
                prefer_converged_over_stub(local, better)
                    || converged_has_reference_params_over_bare(local, better)
            };

        if let Some(ref better) = global_upgraded {
            if resolved_from_mc.as_ref().is_none_or(|local| {
                !is_usable(local) || prefer_global_over(local, better)
            }) {
                if trace {
                    eprintln!(
                        "[wj-sig] call-site {method} arg#{}: global SELECTED ({:?})",
                        arguments.len(),
                        better.param_types
                    );
                }
                return Some(better.clone());
            }
        }

        if let Some(sig) = resolved_from_mc {
            if is_usable(sig) {
                if let Some(ref better) = global_upgraded {
                    if prefer_converged_over_stub(sig, better)
                        || converged_has_reference_params_over_bare(sig, better)
                    {
                        if trace {
                            eprintln!(
                                "[wj-sig] call-site {method} arg#{}: global UPGRADED ({:?})",
                                arguments.len(),
                                better.param_types
                            );
                        }
                        return Some(better.clone());
                    }
                }
                if let Some(ref mc_sig) = mc_resolved {
                    if prefer_converged_over_stub(sig, mc_sig)
                        || converged_has_reference_params_over_bare(sig, mc_sig)
                    {
                        if trace {
                            eprintln!(
                                "[wj-sig] call-site {method} arg#{}: mc_resolve UPGRADED ({:?})",
                                arguments.len(),
                                mc_sig.param_types
                            );
                        }
                        return Some(finalize_call_site_signature(mc_sig.clone()));
                    }
                }
                if trace {
                    eprintln!(
                        "[wj-sig] call-site {method} arg#{}: mc_resolve (types={:?} own={:?} name={})",
                        arguments.len(),
                        sig.param_types,
                        sig.param_ownership,
                        sig.name
                    );
                }
                return Some(finalize_call_site_signature(sig.clone()));
            }
        }

        if let Some(ref better) = global_upgraded {
            if mc_resolved
                .as_ref()
                .is_none_or(|local| prefer_global_over(local, better))
            {
                if trace {
                    eprintln!(
                        "[wj-sig] call-site {method} arg#{}: global over mc_resolved ({:?})",
                        arguments.len(),
                        better.param_types
                    );
                }
                return Some(better.clone());
            }
        }

        if let Some(sig) = mc_resolved {
            return Some(finalize_call_site_signature(sig));
        }

        receiver_type_name
            .as_ref()
            .and_then(|tn| {
                self.resolve_call_signature_with_global(
                    &format!("{tn}::{method}"),
                    Some(tn.as_str()),
                    arguments.len(),
                )
                .map(|r| finalize_call_site_signature(r.sig))
                .filter(|sig| is_usable(sig))
            })
            .or_else(|| {
                receiver_type_name.as_ref().and_then(|tn| {
                    self.lookup_method_signature_on_receiver_type(tn, method, arguments.len())
                        .map(finalize_call_site_signature)
                })
            })
            .or_else(|| {
                resolved_from_mc
                    .as_ref()
                    .filter(|sig| is_usable(sig))
                    .cloned()
                    .map(finalize_call_site_signature)
            })
    }
}
