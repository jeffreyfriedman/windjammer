//! Bridge analyzer signatures to IR `SafetyType` values.

use crate::analyzer::{FunctionSignature, OwnershipMode};
use crate::ir::node::parser_type_to_base_type;
use crate::ir::safety_type::{BaseType, EffectSet, OwnedType, Region, SafetyType, TaintStatus, ConstEval};
use crate::parser::Type;

pub fn bare_wj_formal_type<'a>(sig: &'a FunctionSignature, param_idx: usize) -> Option<&'a Type> {
    sig.formal_param_type(param_idx).map(|t| match t {
        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
        other => other,
    })
}

/// Build a `SafetyType` from a parser `Type`, extracting ownership from reference
/// wrappers when present and falling back to analyzer ownership mode otherwise.
pub fn safety_type_from_parser_type(ty: &Type, fallback_mode: Option<OwnershipMode>) -> SafetyType {
    let (base, ownership) = match ty {
        Type::Reference(inner) => (
            parser_type_to_base_type(inner),
            OwnedType::Ref(Region::fresh(0)),
        ),
        Type::MutableReference(inner) => (
            parser_type_to_base_type(inner),
            OwnedType::MutRef(Region::fresh(1)),
        ),
        other => {
            let base = parser_type_to_base_type(other);
            let ownership = fallback_mode
                .map(ownership_mode_to_owned)
                .unwrap_or(OwnedType::Owned);
            (base, ownership)
        }
    };

    SafetyType {
        base,
        ownership,
        effects: EffectSet::pure(),
        taint: TaintStatus::Clean,
        const_eval: ConstEval::Runtime,
        exec_mode: None,
    }
}

fn is_bare_vec_type(ty: &Type) -> bool {
    matches!(ty, Type::Vec(_)) || matches!(ty, Type::Parameterized(name, _) if name == "Vec")
}

fn is_bare_map_type(ty: &Type) -> bool {
    match ty {
        Type::Parameterized(name, _) => {
            let base = name.split('<').next().unwrap_or(name.as_str());
            matches!(
                base,
                "HashMap" | "BTreeMap" | "IndexMap" | "Map" | "HashSet" | "BTreeSet" | "Set"
            )
        }
        _ => false,
    }
}

fn is_wj_owned_non_text_bare_formal(ty: &Type) -> bool {
    !is_plain_windjammer_string_type(ty)
        && !crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
        && !crate::codegen::rust::types::is_windjammer_text_type(ty)
        && !matches!(ty, Type::Reference(_) | Type::MutableReference(_))
        && !crate::codegen::rust::type_analysis::is_copy_type(ty)
}

/// Expected safety type for a callee parameter at a call site.
///
/// Prefer converged `param_types` (includes Phase-3 Reference wrap and str_ref `&str`)
/// over bare `formal_param_types` so call-site coercion matches emitted Rust signatures.
///
/// When the registry marks a plain `string` param as `Owned`, that contract wins over a
/// stale `Reference(str)` wrap left from body-inferred borrow analysis.
pub fn safety_type_from_signature_param(sig: &FunctionSignature, param_idx: usize) -> SafetyType {
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::MutBorrowed)
    ) {
        // Analyzer MutBorrowed covers both true `&mut T` and owned `mut T` bindings
        // (field mutation on bare Custom). Prefer codegen emission / owned contract.
        let emits_owned_mut_binding = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
            sig, param_idx,
        ) || (sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
            == Some(false)
            && !sig
                .param_types
                .get(param_idx)
                .is_some_and(|t| matches!(t, Type::MutableReference(_)))
            && !sig
                .formal_param_type(param_idx)
                .is_some_and(|t| matches!(t, Type::MutableReference(_))));
        if !emits_owned_mut_binding {
            if let Some(formal) = sig.formal_param_type(param_idx) {
                return safety_type_from_parser_type(
                    &Type::MutableReference(Box::new(formal.clone())),
                    Some(OwnershipMode::MutBorrowed),
                );
            }
        } else if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
            return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
        }
    }

    // WJ bare non-Copy struct formals: owned API methods pass by value (Rust auto-borrow);
    // readonly comparison helpers with converged borrow stay shared-ref at call sites.
    //
    // Do NOT force Owned when the formal itself is an explicit `&T` / `&mut T`
    // (e.g. stdlib `HashMap::get(&K)`). That case is a true borrow contract, not
    // body-converged wrapping of a bare owned formal (`MemoryEngine::put` pattern).
    if let Some(formal) = bare_wj_formal_type(sig, param_idx) {
        if is_wj_owned_non_text_bare_formal(formal) && !is_bare_vec_type(formal) {
            let formal_is_explicit_ref = sig.formal_param_type(param_idx).is_some_and(|t| {
                matches!(t, Type::Reference(_) | Type::MutableReference(_))
            });
            if !formal_is_explicit_ref {
                let emits_ref = sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx))
                    .copied()
                    == Some(true);
                if !emits_ref {
                    let effective_own =
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                            sig, param_idx,
                        );
                    // Body-converged borrow on bare non-Copy formals (Map keys, mutating
                    // lookups) must emit `&T` at call sites regardless of return type — not
                    // only bool-returning readonly helpers.
                    if matches!(
                        effective_own,
                        OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                    ) {
                        // Codegen recorded owned emission (`other: Lsn`) — beats stale
                        // body-converged Borrowed even for bool-returning helpers (WDB-060).
                        if sig
                            .emitted_rust_ref_params
                            .as_ref()
                            .and_then(|flags| flags.get(param_idx))
                            .copied()
                            == Some(false)
                        {
                            return safety_type_from_parser_type(
                                formal,
                                Some(OwnershipMode::Owned),
                            );
                        }
                        // Fall through to converged borrow handling below.
                    } else {
                        let readonly_compare_helper = sig.return_type.as_ref().is_some_and(|t| {
                            matches!(t, Type::Bool)
                                || matches!(t, Type::Custom(name) if name == "bool")
                        }) && matches!(effective_own, OwnershipMode::Borrowed);
                        if !readonly_compare_helper {
                            return safety_type_from_parser_type(formal, Some(OwnershipMode::Owned));
                        }
                    }
                }
            }
        }
    }

    // Codegen refresh recorded owned Rust formals — beats stale body-converged Reference(T).
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx) {
        if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
            return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
        }
    }

    // Converged bare AST formal (`other: Lsn`) beats stale `Reference(Lsn)` in `param_types`
    // when codegen recorded owned emission (`emitted_rust_ref_params[idx] == false`).
    if sig
        .emitted_rust_ref_params
        .as_ref()
        .and_then(|flags| flags.get(param_idx))
        .copied()
        == Some(false)
    {
        if let Some(formal) = sig.formal_param_types.get(param_idx) {
            if !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                && !is_plain_windjammer_string_type(formal)
                && sig.param_types.get(param_idx).is_some_and(|t| match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => {
                        inner.as_ref() == formal
                    }
                    _ => false,
                })
            {
                return safety_type_from_parser_type(formal, Some(OwnershipMode::Owned));
            }
        }
    }

    // Copy aggregates emit owned formals even when analyzer left Borrowed from field
    // reads (`other: Lsn`). Do this before falling through to shared-ref expected type.
    // Stale `Reference(Lsn)` in formal_param_types is fine — formal gen strips it.
    if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
        let is_copy_aggregate = (crate::codegen::rust::type_analysis::is_copy_type(bare)
            || matches!(
                bare,
                Type::Custom(name) if crate::type_classification::is_known_copy_aggregate(name)
            ))
            && !crate::type_classification::is_copy_pass_by_value_formal(bare);
        let emits_shared_ref = sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
            == Some(true);
        if is_copy_aggregate && !emits_shared_ref && !is_plain_windjammer_string_type(bare) {
            return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
        }
        // Codegen-refreshed owned emission on bare Custom (including user Copy aggregates
        // not yet visible to pure type_analysis) beats stale Borrowed/Reference metadata.
        if matches!(bare, Type::Custom(_))
            && !emits_shared_ref
            && !is_plain_windjammer_string_type(bare)
            && (matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Owned)
            ) || sig
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(param_idx))
                .copied()
                == Some(false))
        {
            return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
        }
    }

    // Vec/Map formals pass by value by default: not Copy, so codegen keeps them owned
    // regardless of body-convergence (which may set param_ownership to Borrowed).
    // Exceptions:
    // - `emitted_rust_ref_params == Some(true)` → shared-ref formal was actually emitted
    // - analyzer Borrowed + `Reference(Vec)` with no owned-emission contract → trust borrow
    //   (cross-crate readonly `upload_svo(svo: Vec)` → `&Vec` at call sites)
    if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
        if is_bare_vec_type(bare) || is_bare_map_type(bare) {
            let ref_emission = sig
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(param_idx))
                .copied();
            if ref_emission == Some(true) {
                // Fall through to Reference / Borrowed handling below.
            } else if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                sig, param_idx,
            ) || ref_emission == Some(false)
            {
                return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
            } else {
                let analyzer_borrows = matches!(
                    sig.param_ownership.get(param_idx),
                    Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                ) && sig.param_types.get(param_idx).is_some_and(|t| {
                    matches!(t, Type::Reference(_) | Type::MutableReference(_))
                });
                if !analyzer_borrows {
                    return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
                }
                // Fall through — treat as shared/mut borrow at the call site.
            }
        }
    }

    // Plain WJ `string` with stale converged `&str` stays owned until codegen confirms emission.
    if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
        sig, param_idx,
    ) && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, param_idx)
        && sig.param_types.get(param_idx).is_some_and(|t| {
            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                || matches!(t, Type::Reference(_) | Type::MutableReference(_))
        })
    {
        return SafetyType {
            base: BaseType::String,
            ownership: OwnedType::Owned,
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        };
    }

    // Registry-emitted shared-ref contract (text and converged `Reference(T)` formals).
    if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, param_idx) {
        if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
            sig, param_idx,
        ) || sig.param_types.get(param_idx).is_some_and(|t| {
            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
        }) {
            return SafetyType {
                base: BaseType::String,
                ownership: OwnedType::Ref(Region::fresh(3)),
                effects: EffectSet::pure(),
                taint: TaintStatus::Clean,
                const_eval: crate::ir::safety_type::ConstEval::Runtime,
                exec_mode: None,
            };
        }
        if let Some(ty) = sig.param_types.get(param_idx) {
            if matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
                return safety_type_from_parser_type(ty, Some(OwnershipMode::Borrowed));
            }
        }
    }

    // Readonly WJ `string` formals emit `&str` only after codegen confirms emission.
    if sig
        .formal_param_type(param_idx)
        .is_some_and(is_plain_windjammer_string_type)
        && matches!(
            sig.param_ownership.get(param_idx),
            Some(OwnershipMode::Borrowed)
        )
        && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, param_idx)
    {
        return SafetyType {
            base: BaseType::String,
            ownership: OwnedType::Ref(Region::fresh(3)),
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        };
    }

    // Copy aggregates: converged `Reference(T)` borrows at call sites (WDB-053 `&through`)
    // only when codegen did NOT emit an owned formal. Owned Copy aggregates (Lsn) pass by
    // value — over-borrow (`&through` into `other: Lsn`) must not be forced here.
    if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
        if crate::codegen::rust::type_analysis::is_copy_type(bare)
            && !crate::type_classification::is_copy_pass_by_value_formal(bare)
            && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx)
            && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, param_idx)
            && sig.param_types.get(param_idx).is_some_and(|t| {
                matches!(t, Type::Reference(_) | Type::MutableReference(_))
            })
        {
            if let Some(ty) = sig.param_types.get(param_idx) {
                return safety_type_from_parser_type(ty, Some(OwnershipMode::Borrowed));
            }
        }
    }

    // Converged readonly formals emit shared borrows at call sites (WDB-049 path field/text).
    let effective_own =
        crate::codegen::rust::call_signature_resolution::effective_param_ownership(sig, param_idx);
    if matches!(effective_own, OwnershipMode::Borrowed)
        && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx)
    {
        if let Some(ty) = sig.param_types.get(param_idx) {
            if crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty) {
                return safety_type_from_parser_type(ty, Some(OwnershipMode::Borrowed));
            }
            if matches!(ty, Type::Reference(_) | Type::MutableReference(_))
                && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                    sig, param_idx,
                )
            {
                return safety_type_from_parser_type(ty, Some(OwnershipMode::Borrowed));
            }
        }
        if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
            if is_plain_windjammer_string_type(bare) {
                return SafetyType {
                    base: BaseType::String,
                    ownership: OwnedType::Ref(Region::fresh(3)),
                    effects: EffectSet::pure(),
                    taint: TaintStatus::Clean,
                    const_eval: ConstEval::Runtime,
                    exec_mode: None,
                };
            }
            if !crate::type_classification::is_copy_pass_by_value_formal(bare)
                && !matches!(bare, Type::Vec(_))
                && !matches!(bare, Type::Parameterized(ref name, _) if name == "Vec")
                && !is_bare_map_type(bare)
            {
                if matches!(bare, Type::Custom(_)) {
                    // Bare Custom: shared-ref when codegen confirmed `&T`, or analyzer
                    // converged Borrowed + Reference(T) for non-Copy (`MemoryEngine::put`).
                    // Owned Copy aggregates (`other: Lsn`) must not inherit stale Borrowed (WDB-060).
                    if sig
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(param_idx))
                        .copied()
                        == Some(true)
                    {
                        return safety_type_from_parser_type(
                            &Type::Reference(Box::new(bare.clone())),
                            Some(OwnershipMode::Borrowed),
                        );
                    }
                    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        sig, param_idx,
                    ) {
                        return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
                    }
                    if sig.param_types.get(param_idx).is_some_and(|t| {
                        matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    }) && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                        sig, param_idx,
                    ) {
                        return safety_type_from_parser_type(
                            &Type::Reference(Box::new(bare.clone())),
                            Some(OwnershipMode::Borrowed),
                        );
                    }
                } else {
                    return safety_type_from_parser_type(
                        &Type::Reference(Box::new(bare.clone())),
                        Some(OwnershipMode::Borrowed),
                    );
                }
            }
        }
    }

    // Copy aggregates with converged borrow ownership emit `&T` at call sites (WDB-053)
    // unless codegen recorded an owned Rust formal (Lsn / Copy aggregates stay by-value).
    if matches!(effective_own, OwnershipMode::Borrowed)
        && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx)
    {
        if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
            if !is_plain_windjammer_string_type(bare)
                && crate::codegen::rust::type_analysis::is_copy_type(bare)
                && !crate::type_classification::is_copy_pass_by_value_formal(bare)
            {
                if let Some(ty) = sig.param_types.get(param_idx) {
                    if matches!(ty, Type::Reference(_) | Type::MutableReference(_))
                        && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                            sig, param_idx,
                        )
                    {
                        return safety_type_from_parser_type(ty, Some(OwnershipMode::Borrowed));
                    }
                }
            }
        }
    }

    // Plain WJ `string` formals pass owned `String` until codegen confirms `&str`.
    if crate::codegen::rust::call_site_borrow::plain_string_formal_passes_owned_at_call_site(
        sig, param_idx,
    ) {
        // Stale Owned metadata must not beat converged `&str` emission.
        if let Some(ty) = sig.param_types.get(param_idx) {
            if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                sig, param_idx,
            ) || crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
                || matches!(ty, Type::Reference(_))
            {
                return SafetyType {
                    base: BaseType::String,
                    ownership: OwnedType::Ref(Region::fresh(3)),
                    effects: EffectSet::pure(),
                    taint: TaintStatus::Clean,
                    const_eval: ConstEval::Runtime,
                    exec_mode: None,
                };
            }
        }
        return SafetyType {
            base: BaseType::String,
            ownership: OwnedType::Owned,
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        };
    }

    // Converged Rust param types (Reference/MutableReference) are the emitted contract.
    if let Some(ty) = sig.param_types.get(param_idx) {
        if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, param_idx)
            && crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
        {
            return SafetyType {
                base: BaseType::String,
                ownership: OwnedType::Ref(Region::fresh(3)),
                effects: EffectSet::pure(),
                taint: TaintStatus::Clean,
                const_eval: ConstEval::Runtime,
                exec_mode: None,
            };
        }
        if matches!(ty, Type::Reference(_) | Type::MutableReference(_))
            && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                sig, param_idx,
            )
        {
            return safety_type_from_parser_type(ty, None);
        }
    }

    if crate::codegen::rust::call_signature_resolution::static_impl_text_borrows_at_call_site(
        sig, param_idx,
    ) {
        return SafetyType {
            base: BaseType::String,
            ownership: OwnedType::Ref(Region::fresh(4)),
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        };
    }

    let effective = crate::codegen::rust::call_signature_resolution::effective_param_ownership(
        sig,
        param_idx,
    );
    let mode = Some(effective);
    if matches!(effective, OwnershipMode::Owned) {
        if let Some(ty) = sig.param_types.get(param_idx) {
            if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                sig, param_idx,
            ) {
                return safety_type_from_parser_type(ty, Some(OwnershipMode::Borrowed));
            }
            if crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty) {
                return safety_type_from_parser_type(ty, Some(OwnershipMode::Borrowed));
            }
        }
        let is_plain_string = sig
            .formal_param_type(param_idx)
            .is_some_and(is_plain_windjammer_string_type)
            || sig
                .param_types
                .get(param_idx)
                .is_some_and(is_plain_windjammer_string_type);
        if is_plain_string {
            if !sig.has_self_receiver {
                // Stale converged Reference(str) without codegen confirmation stays owned.
                if sig.param_types.get(param_idx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        || matches!(t, Type::Reference(_))
                }) {
                    return SafetyType {
                        base: BaseType::String,
                        ownership: OwnedType::Owned,
                        effects: EffectSet::pure(),
                        taint: TaintStatus::Clean,
                        const_eval: ConstEval::Runtime,
                        exec_mode: None,
                    };
                }
                // Readonly free-fn string APIs borrow as &str at call sites (WDB-049 replay_all).
                return SafetyType {
                    base: BaseType::String,
                    ownership: OwnedType::Ref(Region::fresh(3)),
                    effects: EffectSet::pure(),
                    taint: TaintStatus::Clean,
                    const_eval: ConstEval::Runtime,
                    exec_mode: None,
                };
            }
            return SafetyType {
                base: BaseType::String,
                ownership: OwnedType::Owned,
                effects: EffectSet::pure(),
                taint: TaintStatus::Clean,
                const_eval: ConstEval::Runtime,
                exec_mode: None,
            };
        }
        let bare_formal = bare_wj_formal_type(sig, param_idx);
        if let Some(bare) = bare_formal {
            if !is_plain_windjammer_string_type(bare) {
                return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
            }
        }
    }

    if let Some(ty) = sig.param_types.get(param_idx) {
        if !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
            sig, param_idx,
        ) {
            if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx)
            {
                if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
                    return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
                }
            }
            // Stale `Reference(T)` in param_types must not beat codegen-owned bare formals
            // when refresh recorded shared-ref was NOT emitted (WDB-060 `through: Lsn`).
            if sig
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(param_idx))
                .copied()
                == Some(false)
            {
                if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
                    if !is_plain_windjammer_string_type(bare) {
                        return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
                    }
                }
            }
            // Copy aggregates emit owned formals — stale `Reference(Lsn)` must not force Ref.
            if let Some(bare) = bare_wj_formal_type(sig, param_idx) {
                let is_copy_aggregate = (crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
                    || matches!(
                        bare,
                        Type::Custom(name)
                            if crate::type_classification::is_known_copy_aggregate(name)
                    ))
                    && !crate::type_classification::is_copy_pass_by_value_formal(bare);
                if is_copy_aggregate
                    && sig
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(param_idx))
                        .copied()
                        != Some(true)
                {
                    return safety_type_from_parser_type(bare, Some(OwnershipMode::Owned));
                }
            }
        }
        return safety_type_from_parser_type(ty, mode);
    }
    if let Some(ty) = sig.formal_param_type(param_idx) {
        return safety_type_from_parser_type(ty, mode);
    }
    SafetyType {
        base: BaseType::Inferred,
        ownership: OwnedType::Owned,
        effects: EffectSet::pure(),
        taint: TaintStatus::Clean,
        const_eval: ConstEval::Runtime,
        exec_mode: None,
    }
}

pub fn call_site_expects_shared_borrow(sig: &FunctionSignature, param_idx: usize) -> bool {
    matches!(
        safety_type_from_signature_param(sig, param_idx).ownership,
        OwnedType::Ref(_)
    )
}

pub fn call_site_expects_owned_pass(sig: &FunctionSignature, param_idx: usize) -> bool {
    matches!(
        safety_type_from_signature_param(sig, param_idx).ownership,
        OwnedType::Owned
    )
}

pub fn ownership_mode_to_owned(mode: OwnershipMode) -> OwnedType {
    match mode {
        OwnershipMode::Owned => OwnedType::Owned,
        OwnershipMode::Borrowed => OwnedType::Ref(Region::fresh(0)),
        OwnershipMode::MutBorrowed => OwnedType::MutRef(Region::fresh(1)),
    }
}

/// Infer the actual `SafetyType` of a call argument from emitted target text.
///
/// Shared by Go/JavaScript backends that lack full Rust `CodeGenerator` context.
/// Solver-resolved types from [`resolve_call_arg_actual_type`] refine this when available.
pub fn safety_type_from_emit_text(arg_str: &str) -> SafetyType {
    if arg_str.ends_with(".clone()") {
        return SafetyType::owned(BaseType::Inferred);
    }
    if arg_str.starts_with("&mut ") {
        return SafetyType {
            base: BaseType::Inferred,
            ownership: OwnedType::MutRef(Region::fresh(1)),
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        };
    }
    if arg_str.starts_with('&') {
        return SafetyType::borrowed(BaseType::Inferred, Region::fresh(0));
    }
    let trimmed = arg_str.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        return SafetyType::borrowed(BaseType::String, Region::fresh(2));
    }
    SafetyType::owned(BaseType::Inferred)
}

/// Lookup solver-resolved safety type for a binding across all IR functions in a module.
pub fn safety_type_from_ir_binding(
    module: &crate::ir::pipeline::IrModule,
    binding: &str,
) -> Option<SafetyType> {
    for ir_fn in &module.functions {
        if let Some(st) = ir_fn.param_types.get(binding) {
            return Some(st.clone());
        }
        if let Some(st) = ir_fn.local_types.get(binding) {
            return Some(st.clone());
        }
    }
    None
}

/// Resolve actual call-site `SafetyType`: emit-text shape + solver binding types when present.
pub fn resolve_call_arg_actual_type(
    module: &crate::ir::pipeline::IrModule,
    arg_str: &str,
) -> SafetyType {
    let from_emit = safety_type_from_emit_text(arg_str);
    let Some(binding) = simple_binding_from_emit_text(arg_str) else {
        return from_emit;
    };
    let Some(from_ir) = safety_type_from_ir_binding(module, &binding) else {
        return from_emit;
    };
    merge_actual_safety_types(from_emit, from_ir)
}

fn simple_binding_from_emit_text(arg_str: &str) -> Option<String> {
    let mut s = arg_str.trim();
    if let Some(base) = s.strip_suffix(".clone()") {
        s = base;
    }
    if let Some(base) = s.strip_prefix("&mut ") {
        s = base;
    } else if let Some(base) = s.strip_prefix('&') {
        s = base;
    }
    s = s.trim();
    if s.is_empty() || !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some(s.to_string())
}

fn merge_actual_safety_types(emit: SafetyType, ir: SafetyType) -> SafetyType {
    SafetyType {
        base: if ir.base != BaseType::Inferred {
            ir.base
        } else {
            emit.base
        },
        ownership: if !matches!(emit.ownership, OwnedType::Owned) {
            emit.ownership
        } else if ir.ownership != OwnedType::Inferred {
            ir.ownership
        } else {
            emit.ownership
        },
        effects: emit.effects,
        taint: emit.taint,
        const_eval: emit.const_eval,
        exec_mode: emit.exec_mode,
    }
}

/// Map solver-resolved IR ownership back to analyzer `OwnershipMode` for the signature registry.
pub fn owned_type_to_ownership_mode(ownership: &OwnedType) -> OwnershipMode {
    match ownership {
        OwnedType::Owned | OwnedType::Copy => OwnershipMode::Owned,
        OwnedType::Ref(_) => OwnershipMode::Borrowed,
        OwnedType::MutRef(_) => OwnershipMode::MutBorrowed,
        OwnedType::Inferred => OwnershipMode::Owned,
    }
}

/// Write solver-resolved parameter ownership from IR back into the converged registry.
/// Enables cross-file IR lowering and delegation wrappers to see prior modules' contracts.
pub fn sync_ir_ownership_to_registry(
    ir_functions: &[crate::ir::node::IrFunction],
    analyzed: &[crate::analyzer::AnalyzedFunction],
    registry: &mut crate::analyzer::SignatureRegistry,
) {
    use crate::analyzer::OwnershipMode;

    for (ir_fn, af) in ir_functions.iter().zip(analyzed.iter()) {
        let mut lookup_keys = vec![ir_fn.name.clone()];
        if !lookup_keys.contains(&af.decl.name) {
            lookup_keys.push(af.decl.name.clone());
        }
        let Some(base_key) = lookup_keys
            .iter()
            .find(|k| registry.get_signature(k).is_some())
            .cloned()
        else {
            continue;
        };
        let Some(mut sig) = registry.get_signature(&base_key).cloned() else {
            continue;
        };
        for (idx, param) in af.decl.parameters.iter().enumerate() {
            let Some(st) = ir_fn.param_types.get(&param.name) else {
                continue;
            };
            let mode = owned_type_to_ownership_mode(&st.ownership);
            if idx >= sig.param_ownership.len() {
                continue;
            }
            let prior = sig.param_ownership[idx];
            // IR formal lowering may classify bare non-Copy WJ params as Owned even when
            // body analysis converged Borrowed (Map keys). Do not let IR sync clobber
            // promoted/converged borrow metadata — call sites need `&` not `.clone()`.
            if matches!(prior, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                && matches!(mode, OwnershipMode::Owned)
                && sig.formal_param_type(idx).is_some_and(|t| {
                    let bare = match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    crate::codegen::rust::call_signature_resolution::formal_type_honors_converged_borrow(
                        bare,
                    )
                })
            {
                continue;
            }
            sig.param_ownership[idx] = mode;
            if matches!(mode, OwnershipMode::Owned)
                && idx < sig.param_types.len()
                && matches!(
                    sig.param_types[idx],
                    Type::Reference(_) | Type::MutableReference(_)
                )
            {
                if let Type::Reference(inner) | Type::MutableReference(inner) =
                    sig.param_types[idx].clone()
                {
                    sig.param_types[idx] = *inner;
                    if idx < sig.formal_param_types.len() {
                        sig.formal_param_types[idx] = sig.param_types[idx].clone();
                    }
                }
            }
        }
        registry.add_function(base_key, sig);
    }
}

fn is_plain_windjammer_string_type(ty: &Type) -> bool {
    match ty {
        Type::String => true,
        Type::Custom(name) => name == "string",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::FunctionSignature;

    fn sig_with_types(param_types: Vec<Type>, ownership: Vec<OwnershipMode>) -> FunctionSignature {
        FunctionSignature {
            name: "test_fn".into(),
            formal_param_types: param_types.clone(),
            param_types,
            param_ownership: ownership,
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    #[test]
    fn call_site_expected_prefers_converged_reference_wrap() {
        let sig = sig_with_types(
            vec![Type::Reference(Box::new(Type::Custom("str".into())))],
            vec![OwnershipMode::Borrowed],
        );
        let expected = safety_type_from_signature_param(&sig, 0);
        assert!(matches!(expected.ownership, OwnedType::Ref(_)));
        assert!(matches!(expected.base, BaseType::String));
    }

    #[test]
    fn stale_converged_str_ref_on_plain_string_formal_stays_owned() {
        let sig = FunctionSignature {
            name: "bar".into(),
            formal_param_types: vec![Type::String],
            param_types: vec![Type::Reference(Box::new(Type::Custom("str".into())))],
            param_ownership: vec![OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("bool".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let expected = safety_type_from_signature_param(&sig, 0);
        assert!(
            matches!(expected.ownership, OwnedType::Owned),
            "plain string formal must stay owned at call site until &str is emitted"
        );
    }

    #[test]
    fn memory_engine_put_key_owned_despite_body_borrow_convergence() {
        let sig = FunctionSignature {
            name: "MemoryEngine::put".into(),
            formal_param_types: vec![
                Type::Custom("MemoryEngine".into()),
                Type::Custom("Key".into()),
                Type::Custom("Value".into()),
            ],
            param_types: vec![
                Type::Custom("MemoryEngine".into()),
                Type::Reference(Box::new(Type::Custom("Key".into()))),
                Type::Reference(Box::new(Type::Custom("Value".into()))),
            ],
            param_ownership: vec![
                OwnershipMode::Owned,
                OwnershipMode::Borrowed,
                OwnershipMode::Borrowed,
            ],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let key_expected = safety_type_from_signature_param(&sig, 1);
        assert!(
            matches!(key_expected.ownership, OwnedType::Owned),
            "put key formal must pass owned at call site (Rust auto-borrow)"
        );
    }

    #[test]
    fn hashmap_get_explicit_ref_key_formal_stays_borrowed() {
        // Stdlib HashMap::get(&self, key: &K) — formal is Reference(K), not a bare
        // WJ struct with body-converged borrow. Must not force Owned (would clone keys).
        let sig = FunctionSignature {
            name: "HashMap::get".into(),
            formal_param_types: vec![
                Type::Custom("HashMap".into()),
                Type::Reference(Box::new(Type::Custom("K".into()))),
            ],
            param_types: vec![
                Type::Custom("HashMap".into()),
                Type::Reference(Box::new(Type::Custom("K".into()))),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Option(Box::new(Type::Reference(Box::new(
                Type::Custom("V".into()),
            ))))),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let key_expected = safety_type_from_signature_param(&sig, 1);
        assert!(
            matches!(key_expected.ownership, OwnedType::Ref(_)),
            "HashMap::get key must stay borrowed at call site, got {:?}",
            key_expected.ownership
        );
        assert!(call_site_expects_shared_borrow(&sig, 1));
        assert!(!call_site_expects_owned_pass(&sig, 1));
    }

    #[test]
    fn keys_equal_borrows_readonly_key_params() {
        let sig = FunctionSignature {
            name: "keys_equal".into(),
            formal_param_types: vec![Type::Custom("Key".into()), Type::Custom("Key".into())],
            param_types: vec![
                Type::Reference(Box::new(Type::Custom("Key".into()))),
                Type::Reference(Box::new(Type::Custom("Key".into()))),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Bool),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let expected = safety_type_from_signature_param(&sig, 0);
        assert!(matches!(expected.ownership, OwnedType::Ref(_)));
    }

    #[test]
    fn quest_manager_bare_quest_id_param_types_expects_shared_borrow() {
        let sig = FunctionSignature {
            name: "QuestManager::is_quest_active".into(),
            formal_param_types: vec![
                Type::Custom("Self".into()),
                Type::Custom("QuestId".into()),
            ],
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Custom("QuestId".into()),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Bool),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let expected = safety_type_from_signature_param(&sig, 1);
        assert!(
            matches!(expected.ownership, OwnedType::Ref(_)),
            "converged QuestId borrow must encode as Ref, got {:?}",
            expected.ownership
        );
        assert!(call_site_expects_shared_borrow(&sig, 1));
    }

    #[test]
    fn free_fn_plain_string_borrows_at_call_site() {
        let sig = FunctionSignature {
            name: "replay_all".into(),
            formal_param_types: vec![Type::String],
            param_types: vec![Type::String],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: Some(Type::Parameterized("Vec".into(), vec![])),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let expected = safety_type_from_signature_param(&sig, 0);
        assert!(matches!(expected.ownership, OwnedType::Ref(_)));
    }

    #[test]
    fn vec_formal_owned_at_call_site() {
        let sig = FunctionSignature {
            name: "keys_equal".into(),
            formal_param_types: vec![Type::Vec(Box::new(Type::Custom("u8".into())))],
            param_types: vec![Type::Reference(Box::new(Type::Vec(Box::new(Type::Custom(
                "u8".into(),
            )))))],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: Some(Type::Bool),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![false]),
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let expected = safety_type_from_signature_param(&sig, 0);
        assert!(matches!(expected.ownership, OwnedType::Owned));
    }

    #[test]
    fn mut_borrowed_bare_custom_with_owned_emission_stays_owned() {
        // LedgerKit: create_export_job mutates deps fields → MutBorrowed analysis, but
        // codegen emits `mut deps: AppDeps` (owned). Call sites must pass by value.
        let sig = FunctionSignature {
            name: "create_export_job".into(),
            formal_param_types: vec![Type::Custom("AppDeps".into())],
            param_types: vec![Type::Custom("AppDeps".into())],
            param_ownership: vec![OwnershipMode::MutBorrowed],
            return_type: Some(Type::Custom("ExportJobView".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![false]),
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let expected = safety_type_from_signature_param(&sig, 0);
        assert!(
            matches!(expected.ownership, OwnedType::Owned),
            "owned mut binding must not force MutRef at call sites"
        );
    }

    #[test]
    fn mut_borrowed_explicit_mut_ref_stays_mut_ref() {
        let sig = FunctionSignature {
            name: "update".into(),
            formal_param_types: vec![Type::MutableReference(Box::new(Type::Custom(
                "PlayerState".into(),
            )))],
            param_types: vec![Type::MutableReference(Box::new(Type::Custom(
                "PlayerState".into(),
            )))],
            param_ownership: vec![OwnershipMode::MutBorrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![false]),
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let expected = safety_type_from_signature_param(&sig, 0);
        assert!(matches!(expected.ownership, OwnedType::MutRef(_)));
    }

    #[test]
    fn readonly_vec_formal_borrows_at_call_site_without_emitted_flags() {
        // Cross-crate: analyzer converged `Vec` → Borrowed + Reference(Vec), but metadata
        // may lack `emitted_rust_ref_params`. Call sites must still pass `&vec`.
        let sig = FunctionSignature {
            name: "VoxelGPURenderer::upload_svo".into(),
            formal_param_types: vec![
                Type::Custom("VoxelGPURenderer".into()),
                Type::Vec(Box::new(Type::Custom("u32".into()))),
            ],
            param_types: vec![
                Type::Custom("VoxelGPURenderer".into()),
                Type::Reference(Box::new(Type::Vec(Box::new(Type::Custom("u32".into()))))),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let expected = safety_type_from_signature_param(&sig, 1);
        assert!(
            matches!(expected.ownership, OwnedType::Ref(_)),
            "readonly Vec formal must borrow at call site, got {:?}",
            expected.ownership
        );
    }

    #[test]
    fn emit_text_borrow_and_binding_lookup() {
        let borrowed = safety_type_from_emit_text("&key");
        assert!(matches!(borrowed.ownership, OwnedType::Ref(_)));

        let owned = safety_type_from_emit_text("key.clone()");
        assert!(matches!(owned.ownership, OwnedType::Owned));
    }
}
