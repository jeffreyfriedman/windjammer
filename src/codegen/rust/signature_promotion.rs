//! Signature promotion and convergence: prefer converged global signatures over
//! per-file declaration stubs and body-inferred borrows at call sites and merge time.

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::Type;

use super::call_signature_resolution::ResolvedSignature;

fn normalize_signature_param_types(types: &[Type]) -> Vec<Type> {
    types
        .iter()
        .map(|t| match t {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref().clone(),
            other => other.clone(),
        })
        .collect()
}

fn arg_count_matches(sig: &FunctionSignature, call_arg_count: usize) -> bool {
    effective_user_arg_count(sig) == call_arg_count
}

fn count_reference_wrapped_params(sig: &FunctionSignature) -> usize {
    sig.param_types
        .iter()
        .enumerate()
        .filter(|(idx, t)| {
            if sig.has_self_receiver && *idx == 0 {
                return false;
            }
            matches!(t, Type::Reference(_) | Type::MutableReference(_))
        })
        .count()
}

/// Phase-3 mirror: when ownership converged to Borrowed/MutBorrowed but `param_types`
/// stayed bare `T` (engine metadata / IR sync), wrap for call-site lowering.
pub(crate) fn wrap_converged_borrow_param_types(sig: &mut FunctionSignature) {
    for idx in 0..sig.param_ownership.len() {
        if sig.has_self_receiver && idx == 0 {
            continue;
        }
        let Some(ty) = sig.param_types.get(idx).cloned() else {
            continue;
        };
        if matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
            continue;
        }
        if crate::codegen::rust::string_utilities::param_is_rust_str_ref(&ty)
            || crate::codegen::rust::types::is_windjammer_text_type(&ty)
        {
            continue;
        }
        if crate::codegen::rust::type_analysis_pure::is_copy_type(&ty) {
            continue;
        }
        match sig.param_ownership.get(idx) {
            Some(OwnershipMode::Borrowed) => {
                sig.param_types[idx] = Type::Reference(Box::new(ty));
            }
            Some(OwnershipMode::MutBorrowed) => {
                sig.param_types[idx] = Type::MutableReference(Box::new(ty));
            }
            _ => {}
        }
    }
}

/// Stale engine/dependency metadata where ownership and param types disagree.
///
/// Examples:
/// - `QuestManager::is_quest_active(self, id: QuestId)` stub marks `id` Owned while impl uses `&QuestId`
/// - Borrowed/MutBorrowed ownership with bare `Custom(T)` instead of `Reference(T)`
///
/// **Not** stale: static helpers that truly consume a param (`MannequinMesh::generate(config: MannequinConfig)`).
pub(crate) fn has_stale_owned_non_copy_params(sig: &FunctionSignature) -> bool {
    sig.param_ownership.iter().enumerate().any(|(idx, own)| {
        if sig.has_self_receiver && idx == 0 {
            return false;
        }
        let Some(ty) = sig.param_types.get(idx) else {
            return false;
        };
        let bare_non_copy = param_type_is_owned_non_text(sig, idx)
            && !matches!(ty, Type::Reference(_) | Type::MutableReference(_))
            && !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(
                ty,
            );
        if sig.emitted_rust_ref_params.as_ref().is_some_and(|flags| {
            flags.get(idx).copied().is_some()
        }) {
            return false;
        }
        match own {
            // Body-converged borrow (Map keys, etc.) is valid even when param_types
            // still show bare `Custom(T)` without `Reference(T)` wrapper.
            OwnershipMode::Borrowed => false,
            // MutBorrowed is a genuine inference from mutation analysis, not a stale
            // stub artifact — never treat it as stale.
            OwnershipMode::MutBorrowed => false,
            // Method args after `self` marked Owned with bare struct type *look like*
            // unrefined engine stubs — but only when the whole signature is still
            // all-Owned. Mixed Borrowed+Owned is real (MemoryEngine::put: `&Key` +
            // owned `Value` consumed by `value_i64`). Filtering those out makes call
            // sites fall back to Owned stubs and emit `key.clone()` into `&Key`.
            OwnershipMode::Owned => {
                if !(sig.has_self_receiver && idx > 0 && bare_non_copy) {
                    return false;
                }
                let has_converged_borrow = sig.param_ownership.iter().enumerate().any(|(i, o)| {
                    if sig.has_self_receiver && i == 0 {
                        return false;
                    }
                    matches!(
                        o,
                        OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                    )
                });
                !has_converged_borrow
            }
        }
    })
}

pub(crate) fn signature_is_declaration_stub_like(sig: &FunctionSignature) -> bool {
    if sig.param_ownership.is_empty() {
        // Zero-argument functions have no ownership to refine — not stubs.
        // Declaration stubs have param_types populated but param_ownership
        // not yet inferred; zero-arg functions have both empty.
        if sig.param_types.is_empty() {
            return false;
        }
        return sig
            .param_types
            .iter()
            .all(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)));
    }
    has_stale_owned_non_copy_params(sig)
}

/// True when `local` still looks like a declaration stub and `global` has converged ownership.
pub fn prefer_converged_over_stub(local: &FunctionSignature, global: &FunctionSignature) -> bool {
    use crate::parser::Type;

    if normalize_signature_param_types(&local.param_types)
        != normalize_signature_param_types(&global.param_types)
    {
        return false;
    }
    if local.param_ownership == global.param_ownership {
        return false;
    }

    // Pattern 1: stub marks all params Owned; convergence introduces borrows (e.g. &mut Grid).
    // Empty param_ownership is a metadata stub — not "all owned" (see Pattern 6).
    let local_all_owned = !local.param_ownership.is_empty()
        && local
            .param_ownership
            .iter()
            .all(|o| matches!(o, OwnershipMode::Owned));
    let global_has_borrow = global
        .param_ownership
        .iter()
        .any(|o| matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed));
    if local_all_owned && global_has_borrow {
        return true;
    }

    // Pattern 2: stub marks string as Borrowed &str; convergence uses owned String.
    let local_stub_str_borrow = local
        .param_ownership
        .iter()
        .zip(&local.param_types)
        .any(|(o, t)| {
            matches!(o, OwnershipMode::Borrowed)
                && matches!(
                    t,
                    Type::Reference(inner) if matches!(inner.as_ref(), Type::Custom(s) if s == "str")
                )
        });
    let global_owned_string =
        global
            .param_ownership
            .iter()
            .zip(&global.param_types)
            .any(|(o, t)| {
                matches!(o, OwnershipMode::Owned)
                    && crate::codegen::rust::string_utilities::param_is_owned_string_type(t)
            });
    if local_stub_str_borrow && global_owned_string {
        return true;
    }

    // Pattern 3: stale dependency metadata marks non-copy args Owned while body analysis
    // converged them to borrowed (often with Reference(T) in param_types). Example:
    // engine QuestManager::is_quest_active(id: Owned QuestId) vs game quest/manager.wj
    // converged (id: Borrowed &QuestId).
    let skip_self = |idx: usize| local.has_self_receiver && idx == 0;
    if local
        .param_ownership
        .iter()
        .enumerate()
        .zip(global.param_ownership.iter())
        .any(|((idx, local_own), global_own)| {
            if skip_self(idx) {
                return false;
            }
            if !matches!(local_own, OwnershipMode::Owned) {
                return false;
            }
            if !matches!(
                global_own,
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
            ) {
                return false;
            }
            local.param_types.get(idx).is_some_and(|t| {
                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    && !crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    && !crate::codegen::rust::types::is_windjammer_text_type(t)
            })
        })
    {
        return true;
    }

    // Pattern 4: body-inferred borrow (local) vs converged owned formal (global).
    // Example: MannequinMesh::generate(config) — impl reads config twice (Borrowed) but
    // the formal consumes by value (Owned). Call sites must pass `config`, not `&config`.
    // Skip when global still looks like a stale engine stub (Pattern 3 inverse).
    if has_stale_owned_non_copy_params(global) {
        return false;
    }
    if local
        .param_ownership
        .iter()
        .enumerate()
        .zip(global.param_ownership.iter())
        .any(|((idx, local_own), global_own)| {
            if skip_self(idx) {
                return false;
            }
            if !matches!(
                local_own,
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
            ) {
                return false;
            }
            if !matches!(global_own, OwnershipMode::Owned) {
                return false;
            }
            global
                .formal_param_type(idx)
                .is_some_and(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)))
        })
    {
        return true;
    }

    // Pattern 5: empty param_ownership (stale engine metadata) vs converged non-empty ownership.
    // Example: engine metadata.json has `MannequinMesh::generate` with `param_ownership: []`
    // while local analysis converged to `[Owned]`. Prefer the converged global.
    // Skip when global only adds body-inferred borrow over a bare owned formal stub.
    if local.param_ownership.is_empty() && !global.param_ownership.is_empty() {
        let skip_self = |idx: usize| local.has_self_receiver && idx == 0;
        let body_borrow_over_owned_stub =
            global
                .param_ownership
                .iter()
                .enumerate()
                .any(|(idx, global_own)| {
                    if skip_self(idx) {
                        return false;
                    }
                    matches!(
                        global_own,
                        OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                    ) && local
                        .param_types
                        .get(idx)
                        .is_some_and(|_| param_type_is_owned_non_text(local, idx))
                });
        if !body_borrow_over_owned_stub {
            return true;
        }
    }

    // Pattern 6: body-inferred borrow on local vs metadata/declaration stub with bare owned formals.
    // Example: local `generate` has `[Borrowed]` from double-use body; global metadata has
    // `param_ownership: []` and `Custom(MannequinConfig)` — call sites must use global/owned formal.
    if global.param_ownership.is_empty() && !local.param_ownership.is_empty() {
        let skip_self = |idx: usize| local.has_self_receiver && idx == 0;
        if local
            .param_ownership
            .iter()
            .enumerate()
            .any(|(idx, local_own)| {
                if skip_self(idx) {
                    return false;
                }
                matches!(
                    local_own,
                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                ) && global
                    .param_types
                    .get(idx)
                    .is_some_and(|_t| param_type_is_owned_non_text(global, idx))
            })
        {
            return true;
        }
    }

    // Pattern 7: Phase 3 wrapped borrowed params as Reference(T) in global; per-file stub still bare.
    // Example: ComponentRegistry::add(data: Vec<u8>) call sites need &data from converged global.
    // Never promote body-inferred borrow over a metadata stub with bare owned formals (MannequinMesh::generate).
    if body_borrow_must_not_replace_owned_formal_stub(local, global) {
        return false;
    }
    if !has_stale_owned_non_copy_params(global)
        && global.param_types.iter().enumerate().any(|(idx, g_ty)| {
            matches!(g_ty, Type::Reference(_) | Type::MutableReference(_))
                && local.param_types.get(idx).is_some_and(|l| {
                    !matches!(l, Type::Reference(_) | Type::MutableReference(_))
                        && normalize_signature_param_types(std::slice::from_ref(l))
                            == normalize_signature_param_types(std::slice::from_ref(g_ty))
                })
        })
    {
        return true;
    }

    false
}

/// Block promotion when body-inferred borrow would overwrite a metadata/declaration stub
/// that still shows a bare owned formal (`param_ownership: []`, `Custom(T)` param type).
pub fn body_borrow_must_not_replace_owned_formal_stub(
    existing: &FunctionSignature,
    converged: &FunctionSignature,
) -> bool {
    if !existing.param_ownership.is_empty() {
        return false;
    }
    let skip_self = |idx: usize| existing.has_self_receiver && idx == 0;
    converged
        .param_ownership
        .iter()
        .enumerate()
        .any(|(idx, converged_own)| {
            if skip_self(idx) {
                return false;
            }
            matches!(
                converged_own,
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
            ) && existing
                .param_types
                .get(idx)
                .is_some_and(|_| param_type_is_owned_non_text(existing, idx))
        })
}

/// Block promotion when body-inferred borrow would overwrite a correct engine/converged
/// owned formal for a Copy struct param (MannequinMesh::generate(config: MannequinConfig)).
pub(crate) fn body_borrow_must_not_replace_owned_copy_formal(
    existing: &FunctionSignature,
    converged: &FunctionSignature,
    copy_structs: &std::collections::HashSet<String>,
) -> bool {
    use crate::parser::Type;

    if existing.param_ownership.is_empty() || converged.param_ownership.is_empty() {
        return false;
    }
    let skip_self = |idx: usize| existing.has_self_receiver && idx == 0;
    existing
        .param_ownership
        .iter()
        .enumerate()
        .zip(converged.param_ownership.iter())
        .any(|((idx, existing_own), converged_own)| {
            if skip_self(idx) {
                return false;
            }
            if !matches!(existing_own, OwnershipMode::Owned) {
                return false;
            }
            if !matches!(
                converged_own,
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
            ) {
                return false;
            }
            existing.param_types.get(idx).is_some_and(|t| {
                matches!(t, Type::Custom(name) if copy_structs.contains(name))
                    && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            })
        })
}

/// True when `global` has body-converged `Reference(str)` where `local` still carries a bare
/// `String` stub for the same parameter (cross-file call sites: world.wj local registry vs
/// component_storage converged global entry).
fn global_has_converged_str_refs_over_local(
    local: &FunctionSignature,
    global: &FunctionSignature,
) -> bool {
    for idx in 0..local
        .param_ownership
        .len()
        .min(global.param_ownership.len())
    {
        if local.has_self_receiver && idx == 0 {
            continue;
        }
        let local_bare_string = local.param_types.get(idx).is_some_and(|t| {
            matches!(t, Type::String)
                || matches!(t, Type::Custom(name) if name == "string" || name == "String")
        });
        let global_str_ref = global
            .param_types
            .get(idx)
            .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref);
        let global_borrowed = matches!(
            global.param_ownership.get(idx),
            Some(OwnershipMode::Borrowed)
        );
        if local_bare_string && global_str_ref && global_borrowed {
            return true;
        }
    }
    false
}

/// Cross-file static impl: global body analysis marked text params `Borrowed` while the caller's
/// local registry still carries declaration stubs (`Owned` + bare `String`).
pub(crate) fn global_has_borrowed_text_over_local_owned_stub(
    local: &FunctionSignature,
    global: &FunctionSignature,
) -> bool {
    if local.has_self_receiver != global.has_self_receiver {
        return false;
    }
    for idx in 0..local
        .param_ownership
        .len()
        .min(global.param_ownership.len())
    {
        if local.has_self_receiver && idx == 0 {
            continue;
        }
        let local_owned_text = matches!(local.param_ownership.get(idx), Some(OwnershipMode::Owned))
            && local.param_types.get(idx).is_some_and(|t| {
                crate::codegen::rust::types::is_windjammer_text_type(t)
                    && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            });
        let global_borrowed_text = matches!(
            global.param_ownership.get(idx),
            Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        ) && global.param_types.get(idx).is_some_and(|t| {
            crate::codegen::rust::types::is_windjammer_text_type(t)
                || crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
        });
        if local_owned_text && global_borrowed_text {
            return true;
        }
    }
    false
}

/// `b` has `Reference(T)` params with `Borrowed`/`MutBorrowed` ownership where `a` has
/// bare `T` (owned stub or body-converged borrow without registry wrap). Indicates `b`
/// was refined by body analysis / Phase-3 promotion and should be preferred.
/// Ignores the self param (idx 0 when `has_self_receiver`).
pub(crate) fn converged_has_reference_params_over_bare(a: &FunctionSignature, b: &FunctionSignature) -> bool {
    let min_len = a.param_ownership.len().min(b.param_ownership.len());
    for idx in 0..min_len {
        if a.has_self_receiver && idx == 0 {
            continue;
        }
        let a_bare = a.param_types.get(idx).is_some_and(|t| {
            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                && !crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
        });
        let a_owned_bare = matches!(a.param_ownership.get(idx), Some(OwnershipMode::Owned)) && a_bare;
        // Multipass: per-caller registry may already show Borrowed ownership while
        // param_types stayed bare `Custom(T)` (engine stub + global merge). Prefer global
        // entries that wrapped the converged borrow as `Reference(T)`.
        let a_borrowed_bare = matches!(
            a.param_ownership.get(idx),
            Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        ) && a_bare;
        let b_borrowed_ref = matches!(
            b.param_ownership.get(idx),
            Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        ) && b
            .param_types
            .get(idx)
            .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
        if (a_owned_bare || a_borrowed_bare) && b_borrowed_ref {
            // Codegen-recorded owned Rust formals (`emitted_rust_ref_params[idx] == false`)
            // must not lose to stale global `Reference(T)` borrow stubs (WDB-060 `other: Lsn`).
            if a.emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(idx))
                .copied()
                == Some(false)
            {
                continue;
            }
            return true;
        }
    }
    false
}

/// Method registry entry was aligned with emitted Rust formals (owned `Key`, etc.)
/// after `refresh_method_registry_from_emitted_formals` — beats stale analyzer borrow.
/// Copy codegen-time `emitted_rust_ref_params` (and aligned param metadata) from an
/// alternate registry entry when call-site resolution picked a stale analysis stub.
pub(crate) fn merge_codegen_refresh_metadata(
    into: &mut FunctionSignature,
    from: &FunctionSignature,
) {
    let Some(ref flags) = from.emitted_rust_ref_params else {
        return;
    };
    into.emitted_rust_ref_params = Some(flags.clone());
    for idx in 0..flags.len().min(into.param_types.len()) {
        match flags.get(idx).copied() {
            Some(false) => {
                // `false` means "not shared `&T`" — either owned or `&mut T`.
                // Prefer `from`'s MutBorrowed / MutableReference over forcing Owned.
                if matches!(
                    from.param_ownership.get(idx),
                    Some(crate::analyzer::OwnershipMode::MutBorrowed)
                ) || matches!(
                    from.param_types.get(idx),
                    Some(Type::MutableReference(_))
                ) {
                    if let Some(Type::MutableReference(inner)) = from.param_types.get(idx) {
                        into.param_types[idx] = Type::MutableReference(inner.clone());
                    } else if let Some(formal) = from.formal_param_type(idx).or_else(|| {
                        into.formal_param_type(idx)
                    }) {
                        let bare = match formal {
                            Type::Reference(inner) | Type::MutableReference(inner) => {
                                inner.as_ref().clone()
                            }
                            other => other.clone(),
                        };
                        into.param_types[idx] = Type::MutableReference(Box::new(bare));
                    }
                    if idx < into.param_ownership.len() {
                        into.param_ownership[idx] = crate::analyzer::OwnershipMode::MutBorrowed;
                    }
                    continue;
                }
                let Some(formal) = into.formal_param_type(idx) else {
                    continue;
                };
                let bare = match formal {
                    Type::Reference(inner) | Type::MutableReference(inner) => *inner.clone(),
                    other => other.clone(),
                };
                if idx < into.param_types.len() {
                    into.param_types[idx] = bare.clone();
                }
                if idx < into.formal_param_types.len() {
                    into.formal_param_types[idx] = bare;
                } else if !into.formal_param_types.is_empty() {
                    into.formal_param_types.push(bare);
                }
                if idx < into.param_ownership.len() {
                    into.param_ownership[idx] = crate::analyzer::OwnershipMode::Owned;
                }
            }
            Some(true) => {
                if let Some(formal) = into.formal_param_type(idx) {
                    if !matches!(
                        into.param_types.get(idx),
                        Some(Type::Reference(_) | Type::MutableReference(_))
                    ) {
                        into.param_types[idx] = Type::Reference(Box::new(formal.clone()));
                    }
                }
                if idx < into.param_ownership.len() {
                    into.param_ownership[idx] = crate::analyzer::OwnershipMode::Borrowed;
                }
            }
            _ => {}
        }
    }
}

/// Merge codegen refresh metadata from the registry when `into` lacks it or is stale.
pub(crate) fn merge_registry_codegen_refresh_if_present(
    into: &mut FunctionSignature,
    registry: &crate::analyzer::SignatureRegistry,
    keys: &[String],
) {
    for key in keys {
        let Some(reg) = registry.get_signature(key) else {
            continue;
        };
        if reg.emitted_rust_ref_params.is_some() {
            merge_codegen_refresh_metadata(into, reg);
            return;
        }
    }
}

pub(crate) fn method_registry_reflects_emitted_owned(sig: &FunctionSignature) -> bool {
    sig.param_ownership.iter().enumerate().any(|(idx, own)| {
        if sig.has_self_receiver && idx == 0 {
            return false;
        }
        emitted_owned_arg_contract(sig, idx)
    })
}

/// Single argument emits as owned non-text in generated Rust (not `&T` / `&mut T`).
pub(crate) fn emitted_owned_arg_contract(sig: &FunctionSignature, param_idx: usize) -> bool {
    // True `&mut T` formals must keep mut-borrow at call sites. Distinguishing that from
    // owned `mut T` bindings (field mutation on bare Custom / AppDeps): the latter records
    // `emitted_rust_ref_params[idx] == false` and keeps a bare formal / param type.
    let param_is_mut_ref = sig
        .param_types
        .get(param_idx)
        .is_some_and(|t| matches!(t, Type::MutableReference(_)));
    let formal_is_explicit_mut_ref = sig
        .formal_param_type(param_idx)
        .is_some_and(|t| matches!(t, Type::MutableReference(_)));
    if param_is_mut_ref || formal_is_explicit_mut_ref {
        return false;
    }
    let analyzer_mut = matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::MutBorrowed)
    );
    if analyzer_mut {
        // Owned `mut deps: AppDeps` after field mutation — codegen refresh says not `&T`.
        if sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
            == Some(false)
        {
            return !param_type_is_borrowed_text(sig, param_idx);
        }
        // No emission record yet: do not claim owned (preserve true `&mut` call sites).
        return false;
    }

    if let Some(ref flags) = sig.emitted_rust_ref_params {
        if flags.get(param_idx).copied().unwrap_or(false) {
            return false;
        }
        if flags.get(param_idx).copied() == Some(false) {
            // Codegen refresh recorded an owned Rust formal; `formal_param_types` may
            // still be stale `Reference(T)` from body-converged analysis.
            return !param_type_is_borrowed_text(sig, param_idx);
        }
    }

    // Copy aggregates always emit owned formals — formal generation strips spurious `&T`
    // from field-read Borrowed analysis (`other: Lsn`). Claiming owned here prevents
    // call sites from emitting `&through` into an owned Lsn formal (WDB-053/060).
    //
    // Bare non-Copy Custom with Borrowed ownership emits `&T` (keys_equal) — do not
    // claim owned for those. MutBorrowed Copy (`&mut PlayerState`) already returned above.
    if let Some(formal) = sig
        .formal_param_type(param_idx)
        .or_else(|| sig.param_types.get(param_idx))
    {
        let bare = match formal {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        let is_copy_aggregate = (crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
            || matches!(
                bare,
                Type::Custom(name)
                    if crate::type_classification::is_known_copy_aggregate(name)
            ))
            && !crate::type_classification::is_copy_pass_by_value_formal(bare);
        // Copy aggregates always emit owned formals — formal generation strips spurious
        // `&T` from field-read Borrowed analysis (`other: Lsn`). Stale `Reference(Lsn)` in
        // formal_param_types must not force call-site `&through` (WDB-060).
        if is_copy_aggregate && !param_type_is_borrowed_text(sig, param_idx) {
            let emits_shared_ref = sig
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(param_idx))
                .copied()
                == Some(true);
            if !emits_shared_ref {
                return true;
            }
        }
        // Bare Custom formal + Owned ownership (codegen refresh) beats stale Reference wrap
        // in param_types when emitted_rust_ref_params records owned emission.
        if matches!(formal, Type::Custom(_))
            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
            && !param_type_is_borrowed_text(sig, param_idx)
            && matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Owned)
            )
            && sig
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(param_idx))
                .copied()
                != Some(true)
        {
            return true;
        }
        // Bare Custom with body-inferred Borrowed still emits owned when formal gen
        // strips Copy-aggregate `&T` (`other: Lsn`). Prefer owned contract whenever the
        // emitted formal is bare Custom and not text — except non-Copy structs that
        // truly demote to `&T` (keys_equal), detected via param_types Reference wrap
        // without an owned emitted_rust_ref_params=false record.
        if matches!(formal, Type::Custom(_))
            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
            && !param_type_is_borrowed_text(sig, param_idx)
            && is_copy_aggregate
        {
            return true;
        }
        if is_copy_aggregate
            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
            && !param_type_is_borrowed_text(sig, param_idx)
        {
            return true;
        }
        let is_bare_custom_struct = matches!(formal, Type::Custom(_));
        let analyzer_borrows = matches!(
            sig.param_ownership.get(param_idx),
            Some(OwnershipMode::Borrowed)
        );
        let param_types_ref = sig.param_types.get(param_idx).is_some_and(|t| {
            matches!(t, Type::Reference(_) | Type::MutableReference(_))
        });
        // Bare Custom + Owned (or Copy aggregate) → owned emission. Body-converged
        // Borrowed on non-Copy Custom (QuestId map-key lookup) must NOT claim owned —
        // call sites need `&QuestId`. Copy aggregates (Lsn) already returned above via
        // `is_copy_aggregate`; codegen-refreshed owned formals use emitted_rust_ref=false.
        // True non-Copy `&T` demotion also wraps `param_types` as Reference or sets
        // emitted_rust_ref=true.
        if is_bare_custom_struct
            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
            && !param_type_is_borrowed_text(sig, param_idx)
            && !param_types_ref
            && (is_copy_aggregate || !analyzer_borrows)
        {
            return true;
        }
        // Bare Custom WJ formal with stale Reference wrap: when codegen recorded owned
        // emission (`emitted_rust_ref_params[idx] == false`) we already returned above.
        // When the flag is absent but ownership was refreshed to Owned, trust owned.
        if is_bare_custom_struct
            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
            && !param_type_is_borrowed_text(sig, param_idx)
            && matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Owned)
            )
        {
            return true;
        }
        if is_bare_custom_struct
            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
            && !param_type_is_borrowed_text(sig, param_idx)
            && !analyzer_borrows
        {
            return true;
        }
    }

    // Non-Copy non-text WJ formals emit as owned in generated Rust when the body
    // actually consumes the param (WDB-055 `engine.put(key: Key)`, WDB-056 Vec<u8>).
    // When body-convergence says Borrowed AND param_types confirms Reference(T),
    // the codegen formal generation emits `&T` — respect that here.
    if let Some(formal) = sig
        .formal_param_type(param_idx)
        .or_else(|| sig.param_types.get(param_idx))
    {
        let bare = match formal {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        let converged_to_ref = sig.param_types.get(param_idx).is_some_and(|t| {
            matches!(t, Type::Reference(_) | Type::MutableReference(_))
        }) && matches!(
            sig.param_ownership.get(param_idx),
            Some(OwnershipMode::Borrowed)
        );
        let is_non_copy_non_text = !crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                sig, param_idx,
            )
            && !crate::codegen::rust::type_analysis::is_copy_type(bare)
            && !matches!(bare, Type::Reference(_) | Type::MutableReference(_))
            && !crate::codegen::rust::string_utilities::param_is_rust_str_ref(bare)
            && !crate::codegen::rust::types::is_windjammer_text_type(bare)
            && !matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::MutBorrowed)
            )
            && !converged_to_ref;
        let is_vec = matches!(bare, Type::Vec(_))
            || matches!(bare, Type::Parameterized(name, _) if name == "Vec");
        // Vec defaults to owned emission (WDB-056), but body-converged `&Vec` / Borrowed
        // must not claim an owned contract — call sites need `&arg` (cross-crate upload_*).
        if is_non_copy_non_text || (is_vec && !converged_to_ref) {
            if matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
            ) && crate::codegen::rust::call_signature_resolution::formal_type_honors_converged_borrow(
                formal,
            ) {
                return false;
            }
            return true;
        }
    }

    if sig.param_types.get(param_idx).is_some_and(|t| {
        matches!(t, Type::Reference(_) | Type::MutableReference(_))
    }) && !sig.emitted_rust_ref_params.as_ref().is_some_and(|flags| {
        flags.get(param_idx).copied() == Some(false)
    }) {
        // Copy aggregates (`Lsn`) emit owned Rust formals even when analyzer left
        // `Reference(T)` without codegen refresh (WDB-060).
        if let Some(bare) = sig.param_types.get(param_idx).map(|t| match t {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        }) {
            if (crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
                || matches!(
                    bare,
                    Type::Custom(name)
                        if crate::type_classification::is_known_copy_aggregate(name)
                ))
                && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                && !param_type_is_borrowed_text(sig, param_idx)
            {
                return true;
            }
        }
        // Bare Custom / Copy-aggregate formals still emit owned even when param_types
        // was promoted to MutableReference by body analysis (handled above).
        return false;
    }
    // Do NOT call effective_param_ownership here — it calls this function (recursion).
    matches!(sig.param_ownership.get(param_idx), Some(OwnershipMode::Owned))
        && param_type_is_owned_non_text(sig, param_idx)
}

fn param_type_is_borrowed_text(sig: &FunctionSignature, param_idx: usize) -> bool {
    sig.formal_param_type(param_idx)
        .or_else(|| sig.param_types.get(param_idx))
        .is_some_and(|t| {
            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                || crate::codegen::rust::types::is_windjammer_text_type(t)
        })
}

/// Global converged `Reference(T)` must not beat a concrete impl whose emitted Rust formals
/// are owned non-text (MemoryEngine::range_scan vs stale trait/body borrow metadata).
pub(crate) fn emitted_owned_beats_stale_global_borrow(
    local: &FunctionSignature,
    global: &FunctionSignature,
) -> bool {
    // Only codegen-refreshed signatures (with emitted_rust_ref_params) beat global
    // converged borrow — not bare declaration stubs with param_ownership: Owned.
    local.emitted_rust_ref_params.is_some()
        && method_registry_reflects_emitted_owned(local)
        && converged_has_reference_params_over_bare(local, global)
}

/// Codegen-refreshed `emitted_rust_ref_params` beats same-key analysis-only metadata
/// (cross-file `ComponentRegistry::register` → `&str` call sites in a later module).
pub(crate) fn codegen_refreshed_beats_analysis_only(
    preferred: &FunctionSignature,
    other: &FunctionSignature,
) -> bool {
    preferred.emitted_rust_ref_params.is_some() && other.emitted_rust_ref_params.is_none()
}

/// Pick the first signature that has codegen `emitted_rust_ref_params`, else the first Some.
///
/// Cross-module free calls (`replay_all(self.path)`) must see the defining module's
/// refreshed `&str` formal rather than a caller-side declaration stub.
pub(crate) fn pick_codegen_refreshed_signature<I>(candidates: I) -> Option<FunctionSignature>
where
    I: IntoIterator<Item = Option<FunctionSignature>>,
{
    let mut first = None;
    let mut refresh_without_shared_ref = None;
    for cand in candidates {
        let Some(sig) = cand else {
            continue;
        };
        if let Some(ref flags) = sig.emitted_rust_ref_params {
            // Prefer defining-module refresh that confirmed at least one `&str`/`&T`
            // slot over importer stubs with all-false emission flags.
            if flags.iter().any(|&f| f) {
                return Some(sig);
            }
            if refresh_without_shared_ref.is_none() {
                refresh_without_shared_ref = Some(sig);
            }
            continue;
        }
        if first.is_none() {
            first = Some(sig);
        }
    }
    refresh_without_shared_ref.or(first)
}

/// True when `preferred` recorded at least one shared-ref formal and `other` did not.
///
/// Importer stubs often keep `emitted_rust_ref_params = Some([false, …])` while the
/// defining module refreshed `&str` slots to `true` — prefer the defining refresh.
pub(crate) fn shared_ref_emission_beats(
    preferred: &FunctionSignature,
    other: &FunctionSignature,
) -> bool {
    let pref_has = preferred
        .emitted_rust_ref_params
        .as_ref()
        .is_some_and(|f| f.iter().any(|&x| x));
    let other_has = other
        .emitted_rust_ref_params
        .as_ref()
        .is_some_and(|f| f.iter().any(|&x| x));
    pref_has && !other_has
}

/// Prefer a defining-module refresh that demoted a WJ `string` formal to `&str` over a
/// stale early `emitted_rust_ref_params = Some([false, …])` (WDB-049 `replay_to_lsn`).
pub(crate) fn prefer_shared_text_ref_signature(
    preferred: Option<FunctionSignature>,
    challenger: Option<&FunctionSignature>,
    param_idx: usize,
) -> Option<FunctionSignature> {
    let Some(challenger) = challenger else {
        return preferred;
    };
    if !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
        challenger, param_idx,
    ) {
        return preferred;
    }
    let Some(pref) = preferred else {
        return Some(challenger.clone());
    };
    if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(&pref, param_idx)
    {
        return Some(pref);
    }
    // Only upgrade stale owned WJ `string` formals — never non-text owned contracts.
    if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
        &pref, param_idx,
    ) {
        return Some(challenger.clone());
    }
    Some(pref)
}

/// Prefer converged global signatures over per-file declaration stubs at call sites.
pub fn pick_best_resolved_signature(
    local: Option<ResolvedSignature>,
    global: Option<ResolvedSignature>,
) -> Option<ResolvedSignature> {
    match (local, global) {
        (Some(l), Some(g)) if emitted_owned_beats_stale_global_borrow(&l.sig, &g.sig) => Some(l),
        (Some(l), Some(g)) if emitted_owned_beats_stale_global_borrow(&g.sig, &l.sig) => Some(g),
        (Some(l), Some(g))
            if g.sig.emitted_rust_ref_params.is_some()
                && l.sig.emitted_rust_ref_params.is_none()
                && converged_has_reference_params_over_bare(&l.sig, &g.sig) =>
        {
            Some(g)
        }
        (Some(l), Some(g)) if codegen_refreshed_beats_analysis_only(&g.sig, &l.sig) => Some(g),
        (Some(l), Some(g)) if codegen_refreshed_beats_analysis_only(&l.sig, &g.sig) => Some(l),
        (Some(l), Some(g))
            if converged_has_reference_params_over_bare(&g.sig, &l.sig)
                && method_registry_reflects_emitted_owned(&g.sig) =>
        {
            Some(g)
        }
        (Some(l), Some(g))
            if (prefer_converged_over_stub(&l.sig, &g.sig)
                || global_has_converged_str_refs_over_local(&l.sig, &g.sig)
                || global_has_borrowed_text_over_local_owned_stub(&l.sig, &g.sig)
                || converged_has_reference_params_over_bare(&l.sig, &g.sig))
                && !emitted_owned_beats_stale_global_borrow(&l.sig, &g.sig) =>
        {
            Some(g)
        }
        (Some(l), Some(g))
            if prefer_converged_over_stub(&g.sig, &l.sig)
                || global_has_converged_str_refs_over_local(&g.sig, &l.sig)
                || global_has_borrowed_text_over_local_owned_stub(&g.sig, &l.sig)
                || (converged_has_reference_params_over_bare(&g.sig, &l.sig)
                    && !method_registry_reflects_emitted_owned(&g.sig)
                    && g.sig.emitted_rust_ref_params.is_none()) =>
        {
            Some(l)
        }
        (Some(l), _) => Some(l),
        (None, Some(g)) => Some(g),
        (None, None) => None,
    }
}

/// Pick the best `Type::method` entry for call-site lowering, preferring converged
/// body analysis over stale engine/dependency stubs on the same receiver.
pub(crate) fn best_method_signature_for_receiver(
    registry: &SignatureRegistry,
    receiver_type: &str,
    method: &str,
    arg_count: usize,
) -> Option<(String, FunctionSignature)> {
    let base = receiver_type.split('<').next().unwrap_or(receiver_type);
    let leaf = base.rsplit("::").next().unwrap_or(base);
    let exact = format!("{base}::{method}");
    let leaf_exact = format!("{leaf}::{method}");
    let suffix = format!("::{base}::{method}");
    let leaf_suffix = format!("::{leaf}::{method}");
    let mut best: Option<(String, FunctionSignature, bool)> = None;

    let mut consider = |key: &str, sig: &FunctionSignature| {
        if !arg_count_matches(sig, arg_count) {
            return;
        }
        if let Some((_, ref best_sig, _)) = best {
            if body_borrow_must_not_replace_owned_formal_stub(best_sig, sig) {
                return;
            }
            if body_borrow_must_not_replace_owned_formal_stub(sig, best_sig) {
                best = Some((key.to_string(), sig.clone(), false));
                return;
            }
        }
        let converged =
            !signature_is_declaration_stub_like(sig) && !has_stale_owned_non_copy_params(sig);
        let sig_emitted = method_registry_reflects_emitted_owned(sig);
        let sig_codegen_refreshed = sig.emitted_rust_ref_params.is_some();
        let str_ref_params = sig
            .param_types
            .iter()
            .filter(|t| crate::codegen::rust::string_utilities::param_is_rust_str_ref(t))
            .count();
        let replace = best.as_ref().is_none_or(|(_, best_sig, prev_converged)| {
            let best_emitted = method_registry_reflects_emitted_owned(best_sig);
            let best_codegen_refreshed = best_sig.emitted_rust_ref_params.is_some();
            if sig_codegen_refreshed && !best_codegen_refreshed {
                return true;
            }
            if !sig_codegen_refreshed && best_codegen_refreshed {
                return false;
            }
            if sig_emitted && !best_emitted {
                return true;
            }
            if !sig_emitted && best_emitted {
                return false;
            }
            if converged && !prev_converged {
                return true;
            }
            if !converged && *prev_converged {
                return false;
            }
            let best_str_refs = best_sig
                .param_types
                .iter()
                .filter(|t| crate::codegen::rust::string_utilities::param_is_rust_str_ref(t))
                .count();
            if str_ref_params > best_str_refs {
                return true;
            }
            if str_ref_params < best_str_refs {
                return false;
            }
            let stale_owned = |s: &FunctionSignature| {
                s.param_ownership
                    .iter()
                    .enumerate()
                    .filter(|(idx, o)| {
                        matches!(o, OwnershipMode::Owned)
                            && super::call_signature_resolution::param_type_is_owned_non_text(
                                s, *idx,
                            )
                    })
                    .count()
            };
            let sig_stale = stale_owned(sig);
            let best_stale = stale_owned(best_sig);
            if sig_stale < best_stale {
                return true;
            }
            if sig_stale > best_stale {
                return false;
            }
            let ref_wraps = count_reference_wrapped_params(sig);
            let best_ref_wraps = count_reference_wrapped_params(best_sig);
            if ref_wraps > best_ref_wraps {
                return true;
            }
            if ref_wraps < best_ref_wraps {
                return false;
            }
            if converged == *prev_converged {
                return key.len() > best.as_ref().unwrap().0.len();
            }
            false
        });
        if replace {
            best = Some((key.to_string(), sig.clone(), converged));
        }
    };

    if let Some(sig) = registry.get_signature(&exact) {
        consider(&exact, sig);
    }
    if leaf != base {
        if let Some(sig) = registry.get_signature(&leaf_exact) {
            consider(&leaf_exact, sig);
        }
    }
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.as_str() == exact
            || key.ends_with(&suffix)
            || key.as_str() == leaf_exact
            || key.ends_with(&leaf_suffix)
        {
            consider(key, sig);
        }
    }

    best.map(|(key, sig, _)| (key, sig))
}

/// User-visible argument count for a signature (call-site arity).
pub(crate) fn effective_user_arg_count(sig: &FunctionSignature) -> usize {
    if !sig.param_ownership.is_empty() {
        if sig.has_self_receiver {
            sig.param_ownership.len().saturating_sub(1)
        } else {
            sig.param_ownership.len()
        }
    } else if sig.has_self_receiver {
        sig.param_types.len().saturating_sub(1)
    } else {
        sig.param_types.len()
    }
}

/// True when the resolved signature's formal param type is an owned non-text value (not `&T`).
pub fn param_type_is_owned_non_text(sig: &FunctionSignature, param_idx: usize) -> bool {
    sig.formal_param_type(param_idx).is_some_and(|t| {
        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            && !crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
            && !crate::codegen::rust::types::is_windjammer_text_type(t)
    })
}
