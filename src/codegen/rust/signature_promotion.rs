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
        if sig.emitted_rust_ref_params.as_ref().and_then(|flags| flags.get(idx).copied())
            == Some(true)
        {
            return false;
        }
        match own {
            // Body-converged borrow (Map keys, etc.) is valid even when param_types
            // still show bare `Custom(T)` without `Reference(T)` wrapper.
            OwnershipMode::Borrowed => false,
            // MutBorrowed is a genuine inference from mutation analysis, not a stale
            // stub artifact — never treat it as stale.
            OwnershipMode::MutBorrowed => false,
            // Method args after `self` marked Owned with bare non-Copy struct type are
            // stale engine/dependency stubs until codegen refresh. Mixed Borrowed+Owned
            // signatures still carry stale Owned slots (engine metadata: Borrowed self +
            // Owned QuestId while the defining module converged to `&QuestId`). Marking
            // those Owned slots stale lets call sites prefer the global converged signature.
            // Real payload-store Owned params (Value on MemoryEngine::put) refresh via
            // codegen `emitted_rust_ref_params[idx] == false` (early return above).
            OwnershipMode::Owned => sig.has_self_receiver && idx > 0 && bare_non_copy,
        }
    })
}

/// Per-param slice of [`has_stale_owned_non_copy_params`]: engine/dependency stubs that
/// mark a method arg after `self` as Owned bare Custom without codegen refresh.
pub(crate) fn param_is_stale_engine_owned_stub(sig: &FunctionSignature, param_idx: usize) -> bool {
    if sig.has_self_receiver && param_idx == 0 {
        return false;
    }
    if sig
        .emitted_rust_ref_params
        .as_ref()
        .and_then(|flags| flags.get(param_idx).copied())
        == Some(true)
    {
        return false;
    }
    let Some(ty) = sig.param_types.get(param_idx) else {
        return false;
    };
    let bare_non_copy = param_type_is_owned_non_text(sig, param_idx)
        && !matches!(ty, Type::Reference(_) | Type::MutableReference(_))
        && !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(
            ty,
        );
    matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Owned)
    ) && sig.has_self_receiver
        && param_idx > 0
        && bare_non_copy
}

/// True when `existing` already has a shared-ref formal that `incoming` would degrade
/// to a bare Owned `Custom` (e.g. stdlib `HashMap::get(&K)` vs collections.wj `key: K`).
///
/// Does **not** block defining-module codegen refresh: analyzer may wrap `key: Key` as
/// `Reference(Key)`+Borrowed from field reads while emission keeps owned `key: Key`
/// (`MemoryEngine::put`). Those refreshes carry `emitted_rust_ref_params`.
pub(crate) fn existing_has_stronger_shared_ref_contract(
    existing: &FunctionSignature,
    incoming: &FunctionSignature,
) -> bool {
    if incoming.emitted_rust_ref_params.is_some()
        && existing.emitted_rust_ref_params.is_none()
    {
        return false;
    }
    if normalize_signature_param_types(&existing.param_types)
        != normalize_signature_param_types(&incoming.param_types)
    {
        return false;
    }
    existing
        .param_ownership
        .iter()
        .enumerate()
        .any(|(idx, own)| {
            if existing.has_self_receiver && idx == 0 {
                return false;
            }
            // Codegen recorded owned emission for this slot — not a stdlib downgrade.
            if incoming
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(idx))
                .copied()
                == Some(false)
            {
                return false;
            }
            let existing_ref = existing
                .param_types
                .get(idx)
                .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)))
                || existing
                    .formal_param_type(idx)
                    .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
            matches!(own, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                && existing_ref
                && incoming
                    .param_ownership
                    .get(idx)
                    .is_some_and(|o| matches!(o, OwnershipMode::Owned))
                && incoming
                    .param_types
                    .get(idx)
                    .is_some_and(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)))
        })
}

/// Restore stdlib `&K` lookup contracts when multipass poisoned them to owned `K`.
///
/// `qualified_name` is the call-site key (`HashMap::get`); `sig.name` may be bare `get`.
pub(crate) fn restore_stdlib_collection_key_contract(
    sig: &mut FunctionSignature,
    qualified_name: Option<&str>,
) {
    let lookup = qualified_name
        .filter(|n| n.contains("::"))
        .unwrap_or(sig.name.as_str());
    let Some(stdlib_sig) = SignatureRegistry::stdlib().get_signature(lookup) else {
        return;
    };
    let receiver_base = lookup.rsplit_once("::").map(|(ty, _)| {
        let bare = ty.rsplit("::").next().unwrap_or(ty);
        bare.split('<').next().unwrap_or(bare)
    });
    let Some(base) = receiver_base else {
        return;
    };
    if !crate::type_classification::is_map_type_name(base)
        && !crate::type_classification::is_set_type_name(base)
    {
        return;
    }
    for idx in 0..stdlib_sig.param_ownership.len() {
        if stdlib_sig.has_self_receiver && idx == 0 {
            continue;
        }
        if !matches!(
            stdlib_sig.param_ownership.get(idx),
            Some(OwnershipMode::Borrowed)
        ) {
            continue;
        }
        let Some(Type::Reference(inner)) = stdlib_sig.param_types.get(idx) else {
            continue;
        };
        let degraded = matches!(sig.param_ownership.get(idx), Some(OwnershipMode::Owned))
            && sig
                .param_types
                .get(idx)
                .is_some_and(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)));
        if !degraded {
            continue;
        }
        if idx < sig.param_ownership.len() {
            sig.param_ownership[idx] = OwnershipMode::Borrowed;
        }
        if idx < sig.param_types.len() {
            sig.param_types[idx] = Type::Reference(inner.clone());
        }
        if idx < sig.formal_param_types.len() {
            sig.formal_param_types[idx] = stdlib_sig
                .formal_param_type(idx)
                .cloned()
                .unwrap_or_else(|| Type::Reference(inner.clone()));
        } else if sig.formal_param_types.is_empty() && idx < sig.param_types.len() {
            // Keep formal list empty — `formal_param_type` falls back to param_types.
        }
    }
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
pub(crate) fn converged_has_reference_params_over_bare(
    a: &FunctionSignature,
    b: &FunctionSignature,
) -> bool {
    let min_len = a.param_ownership.len().min(b.param_ownership.len());
    for idx in 0..min_len {
        if a.has_self_receiver && idx == 0 {
            continue;
        }
        let a_bare = a.param_types.get(idx).is_some_and(|t| {
            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                && !crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
        });
        let a_owned_bare =
            matches!(a.param_ownership.get(idx), Some(OwnershipMode::Owned)) && a_bare;
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
            // must not lose to stale global `Reference(T)` borrow stubs (regression-060 `other: Lsn`).
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
    if let Some(ref string_ref) = from.string_ref_string_formal_params {
        into.string_ref_string_formal_params = Some(string_ref.clone());
    }
    for idx in 0..flags.len().min(into.param_types.len()) {
        match flags.get(idx).copied() {
            Some(false) => {
                // `false` means "not shared `&T`" — either owned or `&mut T`.
                // Prefer `from`'s MutBorrowed / MutableReference over forcing Owned.
                if matches!(
                    from.param_ownership.get(idx),
                    Some(crate::analyzer::OwnershipMode::MutBorrowed)
                ) || matches!(from.param_types.get(idx), Some(Type::MutableReference(_)))
                {
                    if let Some(Type::MutableReference(inner)) = from.param_types.get(idx) {
                        into.param_types[idx] = Type::MutableReference(inner.clone());
                    } else if let Some(formal) = from
                        .formal_param_type(idx)
                        .or_else(|| into.formal_param_type(idx))
                    {
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
                // Prefer `&String` when the refresh source recorded `@string_ref` / Phase-2.
                let string_ref_formal = from
                    .string_ref_string_formal_params
                    .as_ref()
                    .and_then(|f| f.get(idx).copied())
                    == Some(true)
                    || from.param_types.get(idx).is_some_and(
                        crate::codegen::rust::string_utilities::param_is_rust_string_ref,
                    );
                if string_ref_formal {
                    into.param_types[idx] = Type::Reference(Box::new(Type::String));
                    while into.formal_param_types.len() <= idx {
                        into.formal_param_types.push(Type::String);
                    }
                    if idx < into.formal_param_types.len() {
                        into.formal_param_types[idx] = Type::Reference(Box::new(Type::String));
                    }
                } else if let Some(formal) = into.formal_param_type(idx) {
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
    sig.param_ownership.iter().enumerate().any(|(idx, _own)| {
        if sig.has_self_receiver && idx == 0 {
            return false;
        }
        emitted_owned_arg_contract(sig, idx)
    })
}

/// Single argument emits as owned non-text in generated Rust (not `&T` / `&mut T`).
pub(crate) fn emitted_owned_arg_contract(sig: &FunctionSignature, param_idx: usize) -> bool {
    // Stale engine metadata (`Owned QuestId` on `is_quest_active`) must not beat defining-
    // module codegen refresh (`&QuestId`) at call sites.
    if param_is_stale_engine_owned_stub(sig, param_idx) {
        return false;
    }
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
    // Codegen-confirmed emission beats stale analyzer ownership. `false` means the
    // Rust formal is not shared `&T` (owned `T` or `&mut T`). Owned wins here for
    // Copy aggregates kept pass-by-value despite field-read Borrowed analysis
    // (`BatchHandle` / regression-060). True `&mut T` already returned above via
    // MutableReference. Scanner-borrowed formals that emit `&T` record `true`.
    if let Some(ref flags) = sig.emitted_rust_ref_params {
        if flags.get(param_idx).copied().unwrap_or(false) {
            return false;
        }
        if flags.get(param_idx).copied() == Some(false) {
            // `false` means "not shared `&T`" — owned `T` *or* `&mut T`. MutBorrowed
            // without a MutableReference wrap must not claim owned (call sites need
            // `&mut self.field` before formal emission syncs MutableReference).
            if matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::MutBorrowed)
            ) {
                return false;
            }
            // Codegen-confirmed non-ref emission is owned even when analyzer ownership
            // is still Borrowed: plain WJ `string` (`build_html(name: String)`) and bare
            // Custom aggregates (`MemoryEngine::put(key: Key)` after field-read Borrowed).
            // Readonly demotion (`MemoryEngine::get` → `key: &Key`) records
            // `emitted_rust_ref_params[idx] == true`.
            if matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Borrowed)
            ) {
                if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                    sig, param_idx,
                ) {
                    return true;
                }
                if let Some(formal) = sig.formal_param_type(param_idx) {
                    if matches!(formal, Type::Custom(_))
                        && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                        && !crate::codegen::rust::types::is_windjammer_text_type(formal)
                    {
                        return true;
                    }
                }
                // Bare WJ `Vec` / map formals emit owned containers in Rust even when
                // multipass field-read analysis left stale Borrowed (`graph_csr_sort_*`).
                if bare_formal_is_vec_or_map(sig, param_idx) {
                    return true;
                }
                return false;
            }
            return !param_type_is_borrowed_text(sig, param_idx);
        }
    }

    // Shared-ref / runtime-scanner Borrowed contracts without an owned-emission
    // record must not be treated as owned (json::get `&Value`, etc.).
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Borrowed)
    ) {
        // Bare WJ container formals (`Vec`, maps) emit owned Rust params — stale
        // multipass Borrowed must not force `&local` at call sites (wdb-layers CSR).
        if bare_formal_is_vec_or_map(sig, param_idx)
            && !crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                sig, param_idx,
            )
        {
            return true;
        }
        return false;
    }
    let analyzer_mut = matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::MutBorrowed)
    );
    if analyzer_mut {
        // No emission record yet: do not claim owned (preserve true `&mut` call sites).
        return false;
    }

    // Copy aggregates always emit owned formals — formal generation strips spurious `&T`
    // from field-read Borrowed analysis (`other: Lsn`). Claiming owned here prevents
    // call sites from emitting `&through` into an owned Lsn formal (regression-053/060).
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
        // formal_param_types must not force call-site `&through` (regression-060).
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
        let param_types_ref = sig
            .param_types
            .get(param_idx)
            .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
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
    // actually consumes the param (regression-055 `engine.put(key: Key)`, regression-056 Vec<u8>).
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
        let converged_to_ref = sig
            .param_types
            .get(param_idx)
            .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)))
            && matches!(
                sig.param_ownership.get(param_idx),
                Some(OwnershipMode::Borrowed)
            );
        let is_non_copy_non_text =
            !crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                sig, param_idx,
            ) && !crate::codegen::rust::type_analysis::is_copy_type(bare)
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
        // Vec defaults to owned emission (regression-056), but body-converged `&Vec` / Borrowed
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

    if sig
        .param_types
        .get(param_idx)
        .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)))
        && !sig
            .emitted_rust_ref_params
            .as_ref()
            .is_some_and(|flags| flags.get(param_idx).copied() == Some(false))
    {
        // Copy aggregates (`Lsn`) emit owned Rust formals even when analyzer left
        // `Reference(T)` without codegen refresh (regression-060).
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
    matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Owned)
    ) && param_type_is_owned_non_text(sig, param_idx)
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
///
/// When several candidates only have all-false emission flags, prefer the one that
/// records owned emission (`emitted_owned_arg_contract` / Owned ownership) so a stale
/// global Borrowed stub cannot beat a same-module Owned refresh (ReBAC `policy: Policy`).
fn sig_simple_name(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// User-owned refresh beats runtime-std shared-ref when they share a suffix but differ
/// in qualification (`join` vs `strings::join`).
fn owned_user_refresh_beats_stdlib_shared_ref(
    owned: &FunctionSignature,
    shared: &FunctionSignature,
) -> bool {
    method_registry_reflects_emitted_owned(owned)
        && !owned.formal_param_types.is_empty()
        && !signature_is_wj_std_stub_or_runtime_qualified(owned)
        && sig_simple_name(&shared.name) == sig_simple_name(&owned.name)
        && shared.name != owned.name
        && shared
            .emitted_rust_ref_params
            .as_ref()
            .is_some_and(|flags| flags.iter().any(|&f| f))
}

pub(crate) fn pick_codegen_refreshed_signature<I>(candidates: I) -> Option<FunctionSignature>
where
    I: IntoIterator<Item = Option<FunctionSignature>>,
{
    let mut first = None;
    let mut refresh_without_shared_ref = None;
    let mut shared_ref_refresh = None;
    for cand in candidates {
        let Some(sig) = cand else {
            continue;
        };
        if let Some(ref flags) = sig.emitted_rust_ref_params {
            // Prefer defining-module refresh that confirmed at least one `&str`/`&T`
            // slot over importer stubs with all-false emission flags — but defer when a
            // later candidate is a user-owned API clashing on suffix (`join` vs `strings::join`).
            if flags.iter().any(|&f| f) {
                if shared_ref_refresh.is_none() {
                    shared_ref_refresh = Some(sig);
                }
                continue;
            }
            let owned_better = |candidate: &FunctionSignature, incumbent: &FunctionSignature| {
                method_registry_reflects_emitted_owned(candidate)
                    && !method_registry_reflects_emitted_owned(incumbent)
                    || candidate
                        .param_ownership
                        .iter()
                        .filter(|o| matches!(o, OwnershipMode::Owned))
                        .count()
                        > incumbent
                            .param_ownership
                            .iter()
                            .filter(|o| matches!(o, OwnershipMode::Owned))
                            .count()
            };
            match refresh_without_shared_ref {
                None => refresh_without_shared_ref = Some(sig.clone()),
                Some(ref incumbent) if owned_better(&sig, incumbent) => {
                    refresh_without_shared_ref = Some(sig);
                }
                Some(_) => {}
            }
            continue;
        }
        if !sig.formal_param_types.is_empty()
            && !signature_is_wj_std_stub_or_runtime_qualified(&sig)
            && method_registry_reflects_emitted_owned(&sig)
        {
            match refresh_without_shared_ref {
                None => refresh_without_shared_ref = Some(sig.clone()),
                Some(ref incumbent) if method_registry_reflects_emitted_owned(&sig)
                    && !method_registry_reflects_emitted_owned(incumbent) =>
                {
                    refresh_without_shared_ref = Some(sig.clone());
                }
                Some(_) => {}
            }
        }
        if first.is_none() {
            first = Some(sig);
        }
    }
    if let (Some(shared), Some(owned)) = (&shared_ref_refresh, &refresh_without_shared_ref) {
        if owned_user_refresh_beats_stdlib_shared_ref(owned, shared) {
            return Some(owned.clone());
        }
    }
    shared_ref_refresh
        .or(refresh_without_shared_ref)
        .or(first)
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

/// WJ AST declares bare non-text, non-Copy `Custom(T)` (owned API intent).
pub(crate) fn wj_ast_bare_owned_non_text_type(t: &Type) -> bool {
    matches!(t, Type::Custom(_))
        && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
        && !crate::codegen::rust::types::is_windjammer_text_type(t)
        && !crate::codegen::rust::type_analysis_pure::is_copy_type(t)
}

/// Registry slot still records bare WJ `Custom` in `formal_param_types` (import / AST stub).
pub(crate) fn wj_registry_bare_owned_formal_slot(
    sig: &FunctionSignature,
    param_idx: usize,
) -> bool {
    if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, param_idx)
    {
        return false;
    }
    if sig
        .emitted_rust_ref_params
        .as_ref()
        .and_then(|flags| flags.get(param_idx))
        .copied()
        == Some(true)
    {
        return false;
    }
    // Take/restore and mut-passthrough analyze as MutBorrowed / MutableReference while
    // the WJ AST formal stays bare `Custom` — that is not an owned emit contract.
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::MutBorrowed)
    ) || sig
        .param_types
        .get(param_idx)
        .is_some_and(|t| matches!(t, Type::MutableReference(_)))
    {
        return false;
    }
    sig.formal_param_types
        .get(param_idx)
        .is_some_and(wj_ast_bare_owned_non_text_type)
}

pub(crate) fn bare_formal_is_vec_or_map(sig: &FunctionSignature, param_idx: usize) -> bool {
    sig.formal_param_type(param_idx).is_some_and(|t| {
        if matches!(t, Type::Reference(_) | Type::MutableReference(_)) {
            return false;
        }
        matches!(t, Type::Vec(_))
            || matches!(t, Type::Parameterized(name, _) if name == "Vec")
            || matches!(
                t,
                Type::Parameterized(name, _)
                    if name == "HashMap" || name == "Map" || name == "BTreeMap"
            )
    }) && !sig.param_types.get(param_idx).is_some_and(|t| {
        matches!(t, Type::Reference(_) | Type::MutableReference(_))
    })
}

/// Bare WJ user `Custom` formals that emit owned Rust params (not `&T` / `&str`).
pub(crate) fn bare_formal_is_owned_user_type(sig: &FunctionSignature, param_idx: usize) -> bool {
    if crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(sig, param_idx)
    {
        return false;
    }
    if sig.param_types.get(param_idx).is_some_and(|t| {
        matches!(t, Type::Reference(_) | Type::MutableReference(_))
    }) {
        return false;
    }
    sig.formal_param_type(param_idx)
        .is_some_and(|formal| {
            matches!(formal, Type::Custom(_))
                && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                && !crate::codegen::rust::types::is_windjammer_text_type(formal)
        })
}

/// Prefer defining-module refresh with shared-ref emission (`&str`, `&Vec`, …) over a
/// stale call-site stub lacking `emitted_rust_ref_params` confirmation (regression-049).
pub(crate) fn signature_is_wj_std_stub_or_runtime_qualified(sig: &FunctionSignature) -> bool {
    if sig.formal_param_types.is_empty() {
        return true;
    }
    crate::codegen::rust::stdlib_method_traits::callee_path_is_runtime_std(&sig.name)
}

pub(crate) fn prefer_shared_ref_signature(
    preferred: Option<FunctionSignature>,
    challenger: Option<&FunctionSignature>,
    param_idx: usize,
) -> Option<FunctionSignature> {
    let Some(challenger) = challenger else {
        return preferred;
    };
    if let Some(ref pref) = preferred {
        // User-owned API with confirmed WJ formals beats stdlib homonym (`join` vs `strings::join`).
        if !pref.formal_param_types.is_empty()
            && !signature_is_wj_std_stub_or_runtime_qualified(pref)
            && sig_simple_name(&pref.name) == sig_simple_name(&challenger.name)
            && pref.name != challenger.name
            && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                challenger, param_idx,
            )
        {
            return Some(pref.clone());
        }
    }
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
    // Runtime-scanned `&str`/`AsRef<str>` (empty WJ formal_param_types + Reference(str) + emitted)
    // must beat WJ std stubs that recorded owned `String` emission for the same API.
    // Body-converged trait demotions keep a plain WJ `string` formal — those must NOT
    // beat an owned preferred contract (`authenticate(email: string)`).
    //
    // Note: `formal_param_type()` falls back to `param_types` when formals are empty, so
    // runtime scans must be detected via `formal_param_types.is_empty()`, not `is_none()`.
    let challenger_runtime_str_ref = challenger.formal_param_types.is_empty()
        && challenger
            .param_types
            .get(param_idx)
            .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref)
        && challenger
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
            == Some(true);
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(&pref, param_idx) {
        if challenger_runtime_str_ref && signature_is_wj_std_stub_or_runtime_qualified(&pref) {
            return Some(challenger.clone());
        }
        return Some(pref);
    }
    if param_is_stale_engine_owned_stub(&pref, param_idx)
        && crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
            challenger, param_idx,
        )
    {
        return Some(challenger.clone());
    }
    // Prefer a defining-module demotion to `&str`/`&Vec` only when codegen recorded
    // shared-ref emission. Analyzer Borrowed + `&str` on a plain WJ `string` formal
    // (trait methods like `authenticate(email: string)`) must not beat the owned AST
    // contract — otherwise call sites emit `&field` into owned `String` formals.
    // Exception: runtime scanner text refs (above) already returned challenger.
    if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
        &pref, param_idx,
    ) || bare_formal_is_vec_or_map(&pref, param_idx)
    {
        // Do not let body-converged `&str` on the *same* plain WJ formal steal owned
        // trait/user contracts. Only accept challengers that are not also plain-WJ-string
        // formals (runtime / explicit `&str` registry), or that already passed the
        // runtime_str_ref gate above.
        let challenger_codegen_shared = challenger
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
            == Some(true);
        let challenger_also_plain_wj_string =
            crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                challenger, param_idx,
            );
        if challenger_codegen_shared && !challenger_also_plain_wj_string {
            return Some(challenger.clone());
        }
        return Some(pref);
    }
    Some(pref)
}

/// Prefer a defining-module refresh that demoted a WJ `string` formal to `&str` over a
/// stale early `emitted_rust_ref_params = Some([false, …])` (regression-049 `replay_to_lsn`).
pub(crate) fn prefer_shared_text_ref_signature(
    preferred: Option<FunctionSignature>,
    challenger: Option<&FunctionSignature>,
    param_idx: usize,
) -> Option<FunctionSignature> {
    prefer_shared_ref_signature(preferred, challenger, param_idx)
}

/// Merge defining-module codegen refresh into a resolved call signature for borrow lowering.
///
/// Associated calls (`WalSegment::from_bytes`) and free calls both need the defining
/// module's `emitted_rust_ref_params` — not just the caller's import stub.
pub(crate) fn refresh_call_site_signature_for_arg(
    initial: Option<FunctionSignature>,
    callee_name: &str,
    arg_index: usize,
    global: Option<&crate::analyzer::SignatureRegistry>,
    local: &crate::analyzer::SignatureRegistry,
) -> Option<FunctionSignature> {
    let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
    let pidx = initial
        .as_ref()
        .map(|s| s.arg_param_index(arg_index))
        .unwrap_or(arg_index);
    let mut refreshed = pick_codegen_refreshed_signature([
        global.and_then(|g| g.get_signature(callee_name).cloned()),
        global.and_then(|g| g.get_signature(simple).cloned()),
        local.get_signature(callee_name).cloned(),
        local.get_signature(simple).cloned(),
        initial.clone(),
    ]);
    // WJ `.wj` stubs shadow scanned runtime APIs in `signatures`. Challenge with the
    // runtime baseline (`get_fallback_signature`) so `&str`/`AsRef<str>` beat owned
    // `string` emission for literals (`strings::join`, `contains`, `Connection::query`).
    for challenger in [
        global.and_then(|g| g.get_signature(callee_name)),
        global.and_then(|g| g.get_signature(simple)),
        global.and_then(|g| g.get_fallback_signature(callee_name)),
        global.and_then(|g| g.get_fallback_signature(simple)),
        local.get_signature(callee_name),
        local.get_signature(simple),
        local.get_fallback_signature(callee_name),
        local.get_fallback_signature(simple),
    ] {
        refreshed = prefer_shared_ref_signature(refreshed, challenger, pidx);
    }
    let mut out = refreshed.or(initial)?;
    // Trait owned `string` must win over body-converged `&str` after prefer-shared.
    if let Some(g) = global {
        let method = simple;
        crate::codegen::rust::call_signature_resolution::apply_trait_owned_string_call_site_contracts(
            g, method, &mut out,
        );
        out = crate::codegen::rust::call_signature_resolution::finalize_call_site_signature(out);
    }
    Some(out)
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
    let lookup_candidates =
        crate::codegen::rust::stdlib_method_traits::stdlib_receiver_lookup_candidates(
            receiver_type,
        );
    let base = receiver_type.split('<').next().unwrap_or(receiver_type);
    let leaf = base.rsplit("::").next().unwrap_or(base);
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

    for candidate in &lookup_candidates {
        let exact = format!("{candidate}::{method}");
        if let Some(sig) = registry.get_signature(&exact) {
            consider(&exact, sig);
        }
    }
    // Indexed by bare method name — O(matching keys), not O(registry size).
    // Full-map suffix scans blow RSS/time on engine builds (80k+ signatures).
    if let Some(keys) = registry.method_keys_for(method) {
        for key in keys {
            if key.ends_with(&suffix) || key.ends_with(&leaf_suffix) {
                if let Some(sig) = registry.get_signature(key) {
                    consider(key, sig);
                }
            }
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

#[cfg(test)]
mod prefer_shared_runtime_tests {
    use super::*;
    use crate::analyzer::{FunctionSignature, OwnershipMode};
    use crate::parser::Type;

    fn wj_owned_join() -> FunctionSignature {
        FunctionSignature {
            name: "strings::join".into(),
            param_types: vec![Type::Vec(Box::new(Type::String)), Type::String],
            formal_param_types: vec![Type::Vec(Box::new(Type::String)), Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
            return_type: Some(Type::String),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![true, false]),
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    fn runtime_join() -> FunctionSignature {
        FunctionSignature {
            name: "strings::join".into(),
            param_types: vec![
                Type::Reference(Box::new(Type::Vec(Box::new(Type::String)))),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::String),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![true, true]),
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    #[test]
    fn prefer_shared_ref_picks_runtime_connection_query_over_wj_owned_sql() {
        let wj = FunctionSignature {
            name: "Connection::query".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::String,
                Type::Vec(Box::new(Type::String)),
            ],
            formal_param_types: vec![
                Type::Custom("Self".into()),
                Type::String,
                Type::Vec(Box::new(Type::String)),
            ],
            param_ownership: vec![
                OwnershipMode::Borrowed,
                OwnershipMode::Owned,
                OwnershipMode::Owned,
            ],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![true, false, false]),
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let stdlib = crate::analyzer::SignatureRegistry::stdlib();
        let runtime = stdlib
            .get_signature("Connection::query")
            .expect("runtime Connection::query must be scanned into stdlib fallback");
        let sql_idx = wj.arg_param_index(0);
        let merged =
            prefer_shared_ref_signature(Some(wj), Some(runtime), sql_idx).expect("prefer_shared");
        assert!(
            crate::codegen::rust::call_site_borrow::callee_emits_shared_rust_ref_param(
                &merged, sql_idx
            ),
            "runtime AsRef/&str must win over WJ owned sql; got {:?}",
            merged.param_types.get(sql_idx)
        );
        assert!(
            merged
                .param_types
                .get(sql_idx)
                .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref),
            "sql slot must be Reference(str), got {:?}",
            merged.param_types.get(sql_idx)
        );
    }

    #[test]
    fn prefer_shared_ref_picks_runtime_str_over_wj_owned_emission() {
        let preferred = Some(wj_owned_join());
        let runtime = runtime_join();
        let merged = prefer_shared_ref_signature(preferred, Some(&runtime), 1).unwrap();
        assert_eq!(
            merged.emitted_rust_ref_params.as_ref().unwrap()[1],
            true,
            "runtime &str delimiter must beat WJ owned emission"
        );
        assert!(matches!(
            &merged.param_types[1],
            Type::Reference(inner) if matches!(**inner, Type::Custom(ref n) if n == "str")
        ));
    }

    #[test]
    fn dump_strings_join_sigs() {
        let reg = crate::analyzer::SignatureRegistry::stdlib();
        let primary = reg.get_signature("strings::join").expect("join");
        eprintln!(
            "JOIN ownership={:?} emitted={:?} param_types={:?} formal={:?} expects_owned={}",
            primary.param_ownership,
            primary.emitted_rust_ref_params,
            primary.param_types,
            primary.formal_param_types,
            crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &primary, 1
            )
        );
        assert!(
            !crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &primary, 1
            ),
            "stdlib join delimiter must not expect owned"
        );
        assert!(
            !crate::codegen::rust::string_utilities::string_literal_needs_to_string(&primary, 1),
            "string_literal_needs_to_string must be false for join delimiter"
        );
        let expected = crate::ir::signature_bridge::safety_type_from_signature_param(&primary, 1);
        eprintln!("EXPECTED_OWN={:?}", expected.ownership);
        assert!(
            matches!(
                expected.ownership,
                crate::ir::safety_type::OwnedType::Ref(_)
            ),
            "expected Ref for join delimiter, got {:?}",
            expected.ownership
        );
    }

    #[test]
    fn codegen_strings_join_literal_stays_bare() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;

        // Return form: return-owned string coerce must not leak into call args.
        let source = r#"
use std::strings
pub fn exercise(parts: Vec<string>) -> string {
    strings.join(parts, "-")
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(Parser::new(tokens)));
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
        codegen.set_global_signature_registry(std::sync::Arc::new(
            crate::analyzer::SignatureRegistry::stdlib().clone(),
        ));
        let generated = codegen.generate_program(&program, &analyzed);
        assert!(
            !generated.contains("\"-\".to_string()"),
            "join delimiter must stay bare &str (not Vec::join owned). Got:\n{generated}"
        );
        assert!(
            generated.contains("strings::join"),
            "must lower as strings::join, not a stdlib type method. Got:\n{generated}"
        );
    }

    #[test]
    fn refresh_join_delimiter_uses_runtime_fallback_from_stdlib() {
        let reg = crate::analyzer::SignatureRegistry::stdlib();
        // Simulate WJ-owned shadow (meta / analyzed std.wj).
        let mut local =
            crate::analyzer::SignatureRegistry::layered(std::sync::Arc::new(reg.clone()));
        local.add_function("strings::join".into(), wj_owned_join());
        let refreshed = refresh_call_site_signature_for_arg(
            Some(wj_owned_join()),
            "strings::join",
            1,
            Some(&local),
            &local,
        )
        .expect("refresh");
        assert_eq!(
            refreshed
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|f| f.get(1))
                .copied(),
            Some(true),
            "refresh must prefer runtime &str delimiter. Got {:?}",
            refreshed
        );
        assert!(
            !crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &refreshed, 1
            ),
            "delimiter must not expect owned String"
        );
    }

    #[test]
    fn codegen_strings_join_vec_arg_must_not_clone() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;

        let source = r#"
use std::strings
pub fn join_tail(parts: Vec<string>) -> string {
    strings.join(parts, "=")
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(Parser::new(tokens)));
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
        codegen.set_global_signature_registry(std::sync::Arc::new(
            crate::analyzer::SignatureRegistry::stdlib().clone(),
        ));
        let generated = codegen.generate_program(&program, &analyzed);
        assert!(
            !generated.contains("parts.clone()"),
            "Vec into strings::join(&[String]) must not clone (runtime module ≠ Type::method). Got:\n{generated}"
        );
        assert!(
            generated.contains("strings::join(&parts")
                || generated.contains("strings::join(parts,"),
            "expected shared borrow into join. Got:\n{generated}"
        );
    }
}
