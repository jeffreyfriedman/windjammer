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
        match own {
            OwnershipMode::Borrowed => bare_non_copy,
            // MutBorrowed is a genuine inference from mutation analysis, not a stale
            // stub artifact — never treat it as stale.
            OwnershipMode::MutBorrowed => false,
            // Method args after `self` marked Owned with bare struct type are engine stubs.
            OwnershipMode::Owned => sig.has_self_receiver && idx > 0 && bare_non_copy,
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
/// bare `T` with `Owned`. Indicates `b` was refined by body analysis and should be preferred.
/// Ignores the self param (idx 0 when `has_self_receiver`).
pub(crate) fn converged_has_reference_params_over_bare(a: &FunctionSignature, b: &FunctionSignature) -> bool {
    let min_len = a.param_ownership.len().min(b.param_ownership.len());
    for idx in 0..min_len {
        if a.has_self_receiver && idx == 0 {
            continue;
        }
        let a_owned_bare = matches!(a.param_ownership.get(idx), Some(OwnershipMode::Owned))
            && a.param_types
                .get(idx)
                .is_some_and(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)));
        let b_borrowed_ref = matches!(
            b.param_ownership.get(idx),
            Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        ) && b
            .param_types
            .get(idx)
            .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
        if a_owned_bare && b_borrowed_ref {
            return true;
        }
    }
    false
}

/// Method registry entry was aligned with emitted Rust formals (owned `Key`, etc.)
/// after `refresh_method_registry_from_emitted_formals` — beats stale analyzer borrow.
pub(crate) fn method_registry_reflects_emitted_owned(sig: &FunctionSignature) -> bool {
    sig.param_ownership.iter().enumerate().any(|(idx, own)| {
        if sig.has_self_receiver && idx == 0 {
            return false;
        }
        emitted_owned_arg_contract(sig, idx)
    })
}

/// Single argument emits as owned non-text in generated Rust (not `&T`).
pub(crate) fn emitted_owned_arg_contract(sig: &FunctionSignature, param_idx: usize) -> bool {
    if sig.param_types.get(param_idx).is_some_and(|t| {
        matches!(t, Type::Reference(_) | Type::MutableReference(_))
    }) {
        return false;
    }
    if matches!(
        crate::codegen::rust::call_signature_resolution::effective_param_ownership(
            sig, param_idx,
        ),
        OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
    ) {
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

/// Prefer converged global signatures over per-file declaration stubs at call sites.
pub fn pick_best_resolved_signature(
    local: Option<ResolvedSignature>,
    global: Option<ResolvedSignature>,
) -> Option<ResolvedSignature> {
    match (local, global) {
        (Some(l), Some(g)) if emitted_owned_beats_stale_global_borrow(&l.sig, &g.sig) => Some(l),
        (Some(l), Some(g)) if emitted_owned_beats_stale_global_borrow(&g.sig, &l.sig) => Some(g),
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
                    && !method_registry_reflects_emitted_owned(&g.sig)) =>
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
        let str_ref_params = sig
            .param_types
            .iter()
            .filter(|t| crate::codegen::rust::string_utilities::param_is_rust_str_ref(t))
            .count();
        let replace = best.as_ref().is_none_or(|(_, best_sig, prev_converged)| {
            let best_emitted = method_registry_reflects_emitted_owned(best_sig);
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
