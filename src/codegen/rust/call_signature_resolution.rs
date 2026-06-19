//! Unified call signature resolution.
//!
//! Every call expression in the compiler resolves its callee signature through
//! this single module. Resolution follows a strict precedence chain with NO
//! bare unqualified lookups (the root cause of the int-cast collision bug).

use std::collections::HashMap;

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::Type;

#[derive(Debug, Clone)]
pub struct ResolvedSignature {
    pub sig: FunctionSignature,
    pub qualified_key: String,
    pub resolution_method: ResolutionMethod,
    pub has_collision: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionMethod {
    /// `"Vec::push"` matched directly in the registry.
    ExactQualified,
    /// Receiver type provided context: `"{ReceiverType}::{method}"`.
    ReceiverQualified,
    /// Module alias resolved: `"gpu::fn"` → `"gpu_safe::fn"`.
    ModuleAlias,
    /// Progressive qualification: `"a::b::fn"` → tried `"b::fn"`.
    ProgressiveQualified,
    /// Suffix match with arg-count validation (last resort for registry).
    ArgCountValidated,
    /// Converged signature from codegen method registry (`method_signatures_by_type`).
    MethodRegistry,
}

/// Resolve a call signature from the registry.
///
/// Resolution precedence (each step tried only if previous returned `None`):
///
/// 1. **Exact key** — `registry.get_signature(func_name)`
/// 2. **Receiver-qualified** — `"{receiver_type}::{method}"` (and base-type variant)
/// 3. **Identifier-as-qualifier** — for `Foo::bar()` parsed as `Call(FieldAccess)`,
///    try `"{identifier}::{method}"` when identifier differs from receiver_type
/// 4. **Module alias** — resolve alias, retry with resolved qualifier
/// 5. **Progressive qualification** — for `a::b::c`, try `b::c`, then `c` qualified
/// 6. **Arg-count-validated suffix** — `find_signature_by_name_and_arg_count`
/// 7. **None** — caller handles the no-signature case
///
/// **Key invariant**: bare `get_signature("push")` is NEVER attempted.
/// Shared `::`-segment prefix length between caller module and a registry key.
fn module_path_affinity(caller_module: &str, signature_key: &str) -> usize {
    caller_module
        .split("::")
        .zip(signature_key.split("::"))
        .take_while(|(a, b)| a == b)
        .count()
}

fn best_module_qualified_suffix_match(
    registry: &SignatureRegistry,
    suffix: &str,
    arg_count: usize,
    caller_module: Option<&str>,
) -> Option<(String, FunctionSignature)> {
    let mut best: Option<(String, FunctionSignature, usize, usize, bool)> = None;

    let mut consider = |key: &str, sig: &FunctionSignature| {
        if !key.ends_with(suffix) || !validate_arg_count(sig, arg_count) {
            return;
        }
        let converged = !signature_is_declaration_stub_like(sig)
            && !has_stale_owned_non_copy_params(sig);
        let affinity = caller_module
            .map(|caller| module_path_affinity(caller, key))
            .unwrap_or(0);
        let key_len = key.len();
        let replace = best.as_ref().is_none_or(
            |(_, _, best_affinity, best_len, best_converged)| {
                if converged && !best_converged {
                    return true;
                }
                if converged == *best_converged {
                    return affinity > *best_affinity
                        || (affinity == *best_affinity && key_len > *best_len);
                }
                false
            },
        );
        if replace {
            best = Some((key.to_string(), sig.clone(), affinity, key_len, converged));
        }
    };

    for (key, sig) in registry.all_signatures_for_suffix_search() {
        consider(key, sig);
    }
    best.map(|(key, sig, _, _, _)| (key, sig))
}

pub fn resolve_call_signature(
    registry: &SignatureRegistry,
    func_name: &str,
    receiver_type: Option<&str>,
    arg_count: usize,
    module_aliases: &HashMap<String, String>,
    caller_module: Option<&str>,
) -> Option<ResolvedSignature> {
    // Step 1: Exact key match (handles already-qualified names like "Vec::push").
    if let Some(sig) = registry.get_signature(func_name) {
        if validate_arg_count(sig, arg_count) {
            let stub_like = signature_is_declaration_stub_like(sig);
            if !stub_like {
                if let Some(pos) = func_name.rfind("::") {
                    let qualifier = &func_name[..pos];
                    if qualifier.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                        let suffix = format!("::{}", func_name);
                        if let Some((better_key, better_sig)) = best_module_qualified_suffix_match(
                            registry,
                            &suffix,
                            arg_count,
                            caller_module,
                        ) {
                            let exact_affinity = caller_module
                                .map(|m| module_path_affinity(m, func_name))
                                .unwrap_or(0);
                            let better_affinity = caller_module
                                .map(|m| module_path_affinity(m, &better_key))
                                .unwrap_or(0);
                            if better_affinity > exact_affinity
                                || (better_affinity == exact_affinity
                                    && better_key.len() > func_name.len())
                            {
                                return Some(ResolvedSignature {
                                    sig: better_sig,
                                    qualified_key: better_key,
                                    resolution_method: ResolutionMethod::ReceiverQualified,
                                    has_collision: registry.has_collision(func_name),
                                });
                            }
                        }
                    }
                }
                return Some(ResolvedSignature {
                    sig: sig.clone(),
                    qualified_key: func_name.to_string(),
                    resolution_method: ResolutionMethod::ExactQualified,
                    has_collision: registry.has_collision(func_name),
                });
            }
            // Declaration stub with all-Owned params — fall through to module alias /
            // progressive qualification so converged keys (e.g. engine::scene::set_if) win.
        }
        // Key exists but arg count is wrong (often a stale declaration stub).
        // Fall through: module-qualified keys from library multipass may hold the
        // converged signature under a longer prefix (e.g. `foo::Type::method`).
    }

    // Step 2: Receiver-qualified — try `"{receiver_type}::{method}"`.
    let method_part = func_name.rsplit("::").next().unwrap_or(func_name);
    if let Some(recv) = receiver_type {
        if let Some(resolved) = try_receiver_qualified(registry, method_part, recv, arg_count) {
            return Some(resolved);
        }
    }

    // Step 3: Identifier-as-qualifier — for `Emitter::new`, the identifier "Emitter"
    // may be in the func_name even when receiver_type is None or different.
    if let Some(pos) = func_name.rfind("::") {
        let qualifier = &func_name[..pos];

        // Step 3a: Direct qualified name already tried in step 1.
        // Try base-type stripping (e.g., "Vec<i32>::push" → "Vec::push").
        let base_qualifier = qualifier.split('<').next().unwrap_or(qualifier);
        if base_qualifier != qualifier {
            let base_key = format!("{}::{}", base_qualifier, method_part);
            if let Some(sig) = registry.get_signature(&base_key) {
                if validate_arg_count(sig, arg_count) {
                    let has_collision = registry.has_collision(&base_key);
                    return Some(ResolvedSignature {
                        sig: sig.clone(),
                        qualified_key: base_key,
                        resolution_method: ResolutionMethod::ExactQualified,
                        has_collision,
                    });
                }
            }
        }

        // Step 4: Module alias resolution.
        if let Some(original_module) = module_aliases.get(qualifier) {
            let resolved_name = format!("{}::{}", original_module, method_part);
            if let Some(sig) = registry.get_signature(&resolved_name) {
                if validate_arg_count(sig, arg_count) {
                    let has_collision = registry.has_collision(&resolved_name);
                    return Some(ResolvedSignature {
                        sig: sig.clone(),
                        qualified_key: resolved_name,
                        resolution_method: ResolutionMethod::ModuleAlias,
                        has_collision,
                    });
                }
            }
        }

        // Step 5: Progressive qualification for module paths.
        // For `a::b::method`, try `b::method`.
        let parts: Vec<&str> = func_name.split("::").collect();
        if parts.len() > 2 {
            for start in (1..parts.len().saturating_sub(1)).rev() {
                let candidate = parts[start..].join("::");
                if let Some(sig) = registry.get_signature(&candidate) {
                    if validate_arg_count(sig, arg_count) {
                        let has_collision = registry.has_collision(&candidate);
                        return Some(ResolvedSignature {
                            sig: sig.clone(),
                            qualified_key: candidate,
                            resolution_method: ResolutionMethod::ProgressiveQualified,
                            has_collision,
                        });
                    }
                }
            }
        }
    }

    // Step 5b: Collision-aware module-qualified search.
    // When the direct key (e.g., "Ability::activate") has a collision and wrong arg
    // count, search for module-qualified registrations (e.g.,
    // "combat_abilities::Ability::activate") that have the correct arg count.
    // The library multipass registers these longer keys for disambiguation.
    if func_name.contains("::") && registry.has_collision(func_name) {
        let suffix = format!("::{}", func_name);
        for (key, sig) in registry.all_signatures() {
            if key.ends_with(&suffix) && key != func_name && validate_arg_count(sig, arg_count) {
                return Some(ResolvedSignature {
                    sig: sig.clone(),
                    qualified_key: key.clone(),
                    resolution_method: ResolutionMethod::ProgressiveQualified,
                    has_collision: true,
                });
            }
        }
    }

    // Step 5c: Type-qualified suffix search for static calls like `VoxelScene::new(64)`.
    // Stale metadata often registers a bare `VoxelScene::new` with the wrong arity (0 params
    // for the builder-pattern type). The real `quick_start::voxel_scene::VoxelScene::new(i32)`
    // lives under a longer module path — find it before the homonym `::new` arg-count sweep.
    if let Some(pos) = func_name.rfind("::") {
        let qualifier = &func_name[..pos];
        if qualifier.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            let suffix = format!("::{}", func_name);
            if let Some((key, sig)) =
                best_module_qualified_suffix_match(registry, &suffix, arg_count, caller_module)
            {
                return Some(ResolvedSignature {
                    sig,
                    qualified_key: key,
                    resolution_method: ResolutionMethod::ReceiverQualified,
                    has_collision: registry.has_collision(func_name),
                });
            }
        }
    }

    // Step 6: Arg-count-validated match for qualified calls.
    // Type-qualified static calls (`Foo::new`) must not match unrelated homonyms
    // (`Emitter::new`) from the stdlib baseline registry.
    if func_name.contains("::") {
        if let Some(pos) = func_name.rfind("::") {
            let qualifier = &func_name[..pos];
            if qualifier.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                if let Some(sig) =
                    registry.find_method_on_receiver_type(qualifier, method_part, arg_count)
                {
                    let qualified_key = format!("{qualifier}::{method_part}");
                    return Some(ResolvedSignature {
                        sig: sig.clone(),
                        qualified_key,
                        resolution_method: ResolutionMethod::ArgCountValidated,
                        has_collision: registry.has_collision(method_part),
                    });
                }
                return None;
            }
        }
        if let Some(sig) = registry.find_signature_by_name_and_arg_count(method_part, arg_count) {
            let qualified_key = registry
                .signatures
                .iter()
                .find(|(_, v)| std::ptr::eq(*v, sig))
                .map(|(k, _)| k.clone())
                .unwrap_or_else(|| method_part.to_string());
            return Some(ResolvedSignature {
                sig: sig.clone(),
                qualified_key,
                resolution_method: ResolutionMethod::ArgCountValidated,
                has_collision: registry.has_collision(method_part),
            });
        }
    }

    // Step 7: Bare unqualified name with collision — try arg-count disambiguation.
    // For imported free functions like `check_collision(a, b)`, the registry may have
    // multiple entries (e.g., `collision2d::check_collision` with 2 args and
    // `Tilemap::check_collision` with 4 args). Find the one matching our arg count
    // but only for non-self-receiver entries (free functions, not methods).
    if !func_name.contains("::") {
        let suffix = format!("::{}", func_name);
        for (key, sig) in registry.all_signatures() {
            if key.ends_with(&suffix)
                && !sig.has_self_receiver
                && validate_arg_count(sig, arg_count)
            {
                return Some(ResolvedSignature {
                    sig: sig.clone(),
                    qualified_key: key.clone(),
                    resolution_method: ResolutionMethod::ArgCountValidated,
                    has_collision: true,
                });
            }
        }
    }

    None
}

/// Pick the best `Type::method` entry for call-site lowering, preferring converged
/// body analysis over stale engine/dependency stubs on the same receiver.
fn best_method_signature_for_receiver(
    registry: &SignatureRegistry,
    receiver_type: &str,
    method: &str,
    arg_count: usize,
) -> Option<(String, FunctionSignature)> {
    let base = receiver_type.split('<').next().unwrap_or(receiver_type);
    let exact = format!("{base}::{method}");
    let suffix = format!("::{base}::{method}");
    let mut best: Option<(String, FunctionSignature, bool)> = None;

    let mut consider = |key: &str, sig: &FunctionSignature| {
        if !validate_arg_count(sig, arg_count) {
            return;
        }
        let converged = !signature_is_declaration_stub_like(sig)
            && !has_stale_owned_non_copy_params(sig);
        let replace = best.as_ref().is_none_or(|(_, _, prev_converged)| {
            if converged && !prev_converged {
                return true;
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
    for (key, sig) in registry.all_signatures_for_suffix_search() {
        if key.as_str() == exact || key.ends_with(&suffix) {
            consider(key, sig);
        }
    }

    best.map(|(key, sig, _)| (key, sig))
}

/// Try receiver-type-qualified lookup with base-type stripping.
fn try_receiver_qualified(
    registry: &SignatureRegistry,
    method: &str,
    receiver_type: &str,
    arg_count: usize,
) -> Option<ResolvedSignature> {
    let base = receiver_type.split('<').next().unwrap_or(receiver_type);
    if let Some((qualified_key, sig)) =
        best_method_signature_for_receiver(registry, base, method, arg_count)
    {
        let has_collision = registry.has_collision(&qualified_key);
        return Some(ResolvedSignature {
            sig,
            qualified_key,
            resolution_method: ResolutionMethod::ReceiverQualified,
            has_collision,
        });
    }

    if base != receiver_type {
        if let Some((qualified_key, sig)) =
            best_method_signature_for_receiver(registry, receiver_type, method, arg_count)
        {
            let has_collision = registry.has_collision(&qualified_key);
            return Some(ResolvedSignature {
                sig,
                qualified_key,
                resolution_method: ResolutionMethod::ReceiverQualified,
                has_collision,
            });
        }
    }

    None
}

/// Validate that a signature's expected argument count matches the call site.
pub(crate) fn validate_arg_count(sig: &FunctionSignature, call_arg_count: usize) -> bool {
    let expected = effective_user_arg_count(sig);
    expected == call_arg_count
}

fn normalize_signature_param_types(types: &[Type]) -> Vec<Type> {
    types
        .iter()
        .map(|t| match t {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref().clone(),
            other => other.clone(),
        })
        .collect()
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
            OwnershipMode::Borrowed | OwnershipMode::MutBorrowed => bare_non_copy,
            // Method args after `self` marked Owned with bare struct type are engine stubs.
            OwnershipMode::Owned => sig.has_self_receiver && idx > 0 && bare_non_copy,
        }
    })
}

fn signature_is_declaration_stub_like(sig: &FunctionSignature) -> bool {
    if sig.param_ownership.is_empty() {
        return sig.param_types.iter().all(|t| {
            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
        });
    }
    has_stale_owned_non_copy_params(sig)
}

/// True when `local` still looks like a declaration stub and `global` has converged ownership.
pub(crate) fn prefer_converged_over_stub(
    local: &FunctionSignature,
    global: &FunctionSignature,
) -> bool {
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
    let local_all_owned = local
        .param_ownership
        .iter()
        .all(|o| matches!(o, OwnershipMode::Owned));
    let global_has_borrow = global.param_ownership.iter().any(|o| {
        matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
    });
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
    let global_owned_string = global
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
    local
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
}

/// Prefer converged global signatures over per-file declaration stubs at call sites.
pub(crate) fn pick_best_resolved_signature(
    local: Option<ResolvedSignature>,
    global: Option<ResolvedSignature>,
) -> Option<ResolvedSignature> {
    match (local, global) {
        (Some(l), Some(g)) if prefer_converged_over_stub(&l.sig, &g.sig) => Some(g),
        (Some(l), _) => Some(l),
        (None, Some(g)) => Some(g),
        (None, None) => None,
    }
}

/// When a per-file stub says `Owned`, look for a longer module-qualified global key
/// (e.g. `dep::module::touch_grid`) with converged borrow ownership.
pub(crate) fn global_suffix_param_ownership(
    global: &SignatureRegistry,
    func_name: &str,
    arg_count: usize,
    arg_idx: usize,
) -> Option<OwnershipMode> {
    let method = func_name.rsplit("::").next().unwrap_or(func_name);
    let suffix = format!("::{method}");
    let mut best: Option<(usize, OwnershipMode)> = None;
    for (key, sig) in global.all_signatures() {
        if key.ends_with(&suffix) && validate_arg_count(sig, arg_count) {
            if let Some(own) = sig.param_ownership_for_arg(arg_idx) {
                let key_len = key.len();
                if best.as_ref().is_none_or(|(best_len, _)| key_len > *best_len) {
                    best = Some((key_len, *own));
                }
            }
        }
    }
    best.map(|(_, own)| own)
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

/// Resolve callee parameter ownership for call-site lowering.
///
/// When `param_ownership` is populated (converged analysis), trust it over
/// `Reference(T)` wrappers in `param_types` that may lag from Phase 3.
/// Empty `param_ownership` falls back to reference param types (metadata stubs).
pub fn effective_param_ownership(sig: &FunctionSignature, param_idx: usize) -> OwnershipMode {
    if let Some(ty) = sig.param_types.get(param_idx) {
        if crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty) {
            return OwnershipMode::Borrowed;
        }
    }
    if !sig.param_ownership.is_empty() {
        return sig
            .param_ownership
            .get(param_idx)
            .copied()
            .unwrap_or(OwnershipMode::Owned);
    }
    if let Some(ty) = sig.param_types.get(param_idx) {
        match ty {
            Type::Reference(_) => return OwnershipMode::Borrowed,
            Type::MutableReference(_) => return OwnershipMode::MutBorrowed,
            _ => {}
        }
    }
    OwnershipMode::Owned
}

/// `station_builder::set_if`, not `Vec3::new` or bare `helper`.
pub(crate) fn is_external_module_qualified_call(func_name: &str) -> bool {
    func_name.contains("::") && func_name.chars().next().is_some_and(|c| c.is_lowercase())
}

/// True when the resolved signature's param type is an owned non-text value (not `&T`).
pub fn param_type_is_owned_non_text(sig: &FunctionSignature, param_idx: usize) -> bool {
    sig.param_types
        .get(param_idx)
        .is_some_and(|t| {
            !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                && !crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                && !crate::codegen::rust::types::is_windjammer_text_type(t)
        })
}

pub fn effective_param_ownership_for_arg(sig: &FunctionSignature, arg_index: usize) -> OwnershipMode {
    let idx = sig.arg_param_index(arg_index);
    effective_param_ownership(sig, idx)
}

/// Whether an arg-count-validated resolution is safe for a known receiver type.
pub fn arg_count_validated_matches_receiver(
    qualified_key: &str,
    receiver_type: &str,
    method: &str,
) -> bool {
    let exact = format!("{receiver_type}::{method}");
    if qualified_key == exact {
        return true;
    }
    qualified_key.ends_with(&format!("::{exact}"))
}

/// Accept a resolved signature for method-call lowering on `receiver_type`.
pub fn accept_method_resolution_for_receiver(
    resolved: &ResolvedSignature,
    receiver_type: &str,
    method: &str,
) -> bool {
    match resolved.resolution_method {
        ResolutionMethod::ArgCountValidated => {
            arg_count_validated_matches_receiver(&resolved.qualified_key, receiver_type, method)
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{OwnershipMode, SignatureRegistry};
    use crate::parser::Type;

    fn make_sig(name: &str, param_count: usize, has_self: bool) -> FunctionSignature {
        FunctionSignature {
            name: name.to_string(),
            param_types: vec![Type::Custom("i32".into()); param_count],
            param_ownership: vec![OwnershipMode::Owned; param_count + if has_self { 1 } else { 0 }],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: has_self,
            is_extern: false,
        }
    }

    fn make_sig_with_types(name: &str, types: Vec<Type>, has_self: bool) -> FunctionSignature {
        let ownership_len = types.len() + if has_self { 1 } else { 0 };
        FunctionSignature {
            name: name.to_string(),
            param_types: types,
            param_ownership: vec![OwnershipMode::Owned; ownership_len],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: has_self,
            is_extern: false,
        }
    }

    fn empty_aliases() -> HashMap<String, String> {
        HashMap::new()
    }

    #[test]
    fn exact_qualified_match() {
        let mut reg = SignatureRegistry::new();
        reg.add_function("Vec::push".into(), make_sig("push", 1, true));

        let result = resolve_call_signature(&reg, "Vec::push", None, 1, &empty_aliases(), None);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.resolution_method, ResolutionMethod::ExactQualified);
        assert_eq!(r.qualified_key, "Vec::push");
    }

    #[test]
    fn receiver_qualified_match() {
        let mut reg = SignatureRegistry::new();
        reg.add_function("Emitter::new".into(), make_sig("new", 2, false));

        let result = resolve_call_signature(&reg, "new", Some("Emitter"), 2, &empty_aliases(), None);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.resolution_method, ResolutionMethod::ReceiverQualified);
    }

    #[test]
    fn bare_name_never_matches_wrong_type() {
        let mut reg = SignatureRegistry::new();
        // Vec3::new takes 3 f32 args
        reg.add_function(
            "Vec3::new".into(),
            make_sig_with_types(
                "new",
                vec![
                    Type::Custom("f32".into()),
                    Type::Custom("f32".into()),
                    Type::Custom("f32".into()),
                ],
                false,
            ),
        );
        // Emitter::new takes 2 args (Vec3, i32)
        reg.add_function(
            "Emitter::new".into(),
            make_sig_with_types(
                "new",
                vec![Type::Custom("Vec3".into()), Type::Custom("i32".into())],
                false,
            ),
        );

        // Looking up "new" bare with 2 args should NOT match Vec3::new (3 args)
        // and SHOULD match Emitter::new (2 args) via arg-count validation
        let result = resolve_call_signature(&reg, "Emitter::new", None, 2, &empty_aliases(), None);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.qualified_key, "Emitter::new");
        assert!(!r
            .sig
            .param_types
            .iter()
            .any(|t| matches!(t, Type::Custom(n) if n == "f32")));
    }

    #[test]
    fn module_alias_resolution() {
        let mut reg = SignatureRegistry::new();
        reg.add_function(
            "gpu_safe::load_shader".into(),
            make_sig("load_shader", 1, false),
        );

        let mut aliases = HashMap::new();
        aliases.insert("gpu".into(), "gpu_safe".into());

        let result = resolve_call_signature(&reg, "gpu::load_shader", None, 1, &aliases, None);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.resolution_method, ResolutionMethod::ModuleAlias);
    }

    #[test]
    fn arg_count_mismatch_rejects() {
        let mut reg = SignatureRegistry::new();
        reg.add_function("Foo::new".into(), make_sig("new", 3, false));

        // Call with 2 args should NOT match a 3-param signature
        let result = resolve_call_signature(&reg, "Foo::new", None, 2, &empty_aliases(), None);
        assert!(result.is_none());
    }

    #[test]
    fn collision_detected() {
        let mut reg = SignatureRegistry::new();
        reg.add_function(
            "Emitter::new".into(),
            make_sig_with_types(
                "new",
                vec![Type::Custom("Vec3".into()), Type::Custom("i32".into())],
                false,
            ),
        );
        reg.add_function(
            "Emitter::new".into(),
            make_sig_with_types(
                "new",
                vec![Type::Custom("f32".into()), Type::Custom("f32".into())],
                false,
            ),
        );

        let result = resolve_call_signature(&reg, "Emitter::new", None, 2, &empty_aliases(), None);
        assert!(result.is_some());
        assert!(result.unwrap().has_collision);
    }

    #[test]
    fn progressive_qualified_match() {
        let mut reg = SignatureRegistry::new();
        reg.add_function(
            "rendering::Camera::update".into(),
            make_sig("update", 1, true),
        );

        let result = resolve_call_signature(
            &reg,
            "scene::rendering::Camera::update",
            None,
            1,
            &empty_aliases(),
            None,
        );
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.resolution_method, ResolutionMethod::ProgressiveQualified);
    }

    #[test]
    fn static_impl_readonly_string_param_is_borrowed_in_registry() {
        use crate::analyzer::Analyzer;
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let source = r#"
impl BuildFingerprint {
    fn collect_wj_files(dir: string) -> Vec<string> {
        Vec::new()
    }

    fn hash_files(files: Vec<string>) -> u64 {
        0
    }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (_, registry, _) = analyzer.analyze_program(&program).expect("analyze");

        let collect = registry
            .get_signature("BuildFingerprint::collect_wj_files")
            .expect("collect_wj_files sig");
        assert_eq!(
            effective_param_ownership(collect, 0),
            OwnershipMode::Borrowed,
            "dir param types={:?} ownership={:?}",
            collect.param_types,
            collect.param_ownership
        );

        let hash = registry
            .get_signature("BuildFingerprint::hash_files")
            .expect("hash_files sig");
        assert_eq!(
            effective_param_ownership(hash, 0),
            OwnershipMode::Borrowed,
            "files param types={:?} ownership={:?}",
            hash.param_types,
            hash.param_ownership
        );

        let resolved = resolve_call_signature(
            &registry,
            "BuildFingerprint::collect_wj_files",
            Some("BuildFingerprint"),
            1,
            &empty_aliases(),
            None,
        );
        assert!(
            resolved.is_some(),
            "qualified static impl method must resolve in registry"
        );
    }

    #[test]
    fn stale_borrowed_metadata_on_owned_struct_param_is_owned() {
        let sig = FunctionSignature {
            name: "svo64_convert::voxelgrid_to_svo64_flat".to_string(),
            param_types: vec![Type::Custom("VoxelGrid".to_string())],
            param_ownership: vec![OwnershipMode::Borrowed],
            return_type: Some(Type::Parameterized(
                "Vec".to_string(),
                vec![Type::Custom("u32".to_string())],
            )),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        };
        assert!(
            param_type_is_owned_non_text(&sig, 0),
            "Custom(VoxelGrid) without Reference is owned at call site"
        );
        assert_eq!(
            effective_param_ownership(&sig, 0),
            OwnershipMode::Borrowed,
            "stale Borrowed in param_ownership still reports Borrowed for legacy paths"
        );
    }

    #[test]
    fn reference_wrapped_struct_param_is_borrowed() {
        let sig = FunctionSignature {
            name: "QuestManager::update_objective_progress".to_string(),
            param_types: vec![Type::Reference(Box::new(Type::Custom("QuestId".to_string())))],
            param_ownership: vec![OwnershipMode::Borrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        };
        assert_eq!(
            effective_param_ownership(&sig, 0),
            OwnershipMode::Borrowed,
        );
        assert!(
            !param_type_is_owned_non_text(&sig, 0),
            "Reference(QuestId) is not owned"
        );
    }

    #[test]
    fn self_static_method_call_emits_borrow_not_to_string() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;

        let source = r#"
impl BuildFingerprint {
    pub fn generate(source_dir: string) -> BuildFingerprint {
        let files = Self::collect_wj_files(source_dir)
        let hash = Self::hash_files(files)
        BuildFingerprint { source_hash: hash, build_timestamp: 0, source_files: files }
    }

    fn collect_wj_files(dir: string) -> Vec<string> {
        Vec::new()
    }

    fn hash_files(files: Vec<string>) -> u64 {
        0
    }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let mut codegen = CodeGenerator::new(registry, CompilationTarget::Rust);
        let rs = codegen.generate_program(&program, &analyzed);

        assert!(
            rs.contains("Self::collect_wj_files(source_dir)")
                || rs.contains("Self::collect_wj_files(&source_dir)"),
            "borrowed string static arg must not to_string. Got:\n{rs}"
        );
        assert!(
            rs.contains("Self::hash_files(&files)") || rs.contains("Self::hash_files(files.as_ref())"),
            "borrowed Vec param must use reference. Got:\n{rs}"
        );
        assert!(
            !rs.contains("hash_files(files.clone())"),
            "must not clone Vec for borrowed param. Got:\n{rs}"
        );
    }

    #[test]
    fn library_preconverged_pass_keeps_borrowed_static_method_params() {
        use crate::analyzer::Analyzer;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use std::sync::Arc;

        let source = r#"
impl BuildFingerprint {
    fn collect_wj_files(dir: string) -> Vec<string> {
        Vec::new()
    }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let global = Arc::new(SignatureRegistry::new());
        let mut analyzer = Analyzer::new();
        analyzer.ownership_preconverged = true;
        let (_, registry, _) = analyzer
            .analyze_program_with_global_arc(&program, &global)
            .expect("analyze");

        let sig = registry
            .get_signature("BuildFingerprint::collect_wj_files")
            .expect("sig");
        assert_eq!(
            effective_param_ownership(sig, 0),
            OwnershipMode::Borrowed,
            "preconverged library pass must still expose borrowed string params; types={:?} ownership={:?}",
            sig.param_types,
            sig.param_ownership
        );
    }

    #[test]
    fn compilation_pipeline_two_pass_static_self_borrows() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::type_inference::{FloatInference, IntInference};
        use crate::CompilationTarget;

        let source = r#"
impl BuildFingerprint {
    pub fn generate(source_dir: string) -> BuildFingerprint {
        let files = Self::collect_wj_files(source_dir)
        let hash = Self::hash_files(files)
        BuildFingerprint { source_hash: hash, build_timestamp: 0, source_files: files }
    }

    fn collect_wj_files(dir: string) -> Vec<string> {
        Vec::new()
    }

    fn hash_files(files: Vec<string>) -> u64 {
        0
    }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");

        let mut global_signatures = SignatureRegistry::new();
        let mut analyzer = Analyzer::new();
        let meta_roots = [std::path::Path::new(".")];
        crate::metadata::merge_wj_meta_signatures_and_copy_structs_multi(
            &meta_roots,
            &mut global_signatures,
            &mut analyzer,
        );
        let (_, first_pass_registry, _) = analyzer
            .analyze_program_with_global_signatures(&program, &global_signatures)
            .expect("pass1");
        global_signatures.merge(&first_pass_registry);

        let copy_structs: std::collections::HashSet<String> =
            analyzer.get_copy_structs().into_iter().collect();
        let mut analyzer_pass2 = Analyzer::new_with_copy_structs(copy_structs);
        let (analyzed, registry, _) = analyzer_pass2
            .analyze_program_with_global_signatures(&program, &global_signatures)
            .expect("pass2");

        let mut float_inference = FloatInference::new();
        float_inference.infer_program(&program);
        let mut int_inference = IntInference::new();
        int_inference.infer_program(&program);

        let mut codegen = CodeGenerator::new(registry, CompilationTarget::Rust);
        codegen.set_float_inference(float_inference);
        codegen.set_int_inference(int_inference);
        crate::compiler::apply_inferred_bounds_to_codegen(&mut codegen, &program);

        let rs = codegen.generate_program(&program, &analyzed);
        assert!(
            !rs.contains("source_dir.to_string()"),
            "two-pass pipeline must not to_string borrowed static string arg. Got:\n{rs}"
        );
        assert!(
            !rs.contains("hash_files(files.clone())"),
            "two-pass pipeline must not clone borrowed Vec arg. Got:\n{rs}"
        );
    }

    #[test]
    fn library_preconverged_codegen_lookup_static_self_method() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::{Expression, Item, Statement};
        use crate::parser::Parser;
        use crate::CompilationTarget;
        use std::sync::Arc;

        let source = r#"
impl BuildFingerprint {
    pub fn generate(source_dir: string) -> BuildFingerprint {
        let files = Self::collect_wj_files(source_dir)
        let hash = Self::hash_files(files)
        BuildFingerprint { source_hash: hash, build_timestamp: 0, source_files: files }
    }

    fn collect_wj_files(dir: string) -> Vec<string> {
        Vec::new()
    }

    fn hash_files(files: Vec<string>) -> u64 {
        0
    }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");

        let stub_registry = SignatureRegistry::from_program_declarations(&program);
        let mut global = SignatureRegistry::new();
        global.merge(&stub_registry);

        let global = Arc::new(global);
        let mut analyzer = Analyzer::for_library_pass(
            Default::default(),
            Default::default(),
            Default::default(),
        );
        analyzer.ownership_preconverged = true;
        let (analyzed, registry, _) = analyzer
            .analyze_program_with_global_arc(&program, &global)
            .expect("analyze");

        let stored = registry
            .get_signature("BuildFingerprint::collect_wj_files")
            .expect("registry key");
        assert_eq!(
            effective_param_ownership(stored, 0),
            OwnershipMode::Borrowed,
            "stored sig types={:?} ownership={:?}",
            stored.param_types,
            stored.param_ownership
        );

        let mut codegen = CodeGenerator::new_for_module(registry.clone(), CompilationTarget::Rust);
        codegen.set_global_signature_registry(global);
        let looked = codegen
            .lookup_method_signature_on_receiver_type("BuildFingerprint", "collect_wj_files", 1)
            .expect("lookup must resolve static impl method");
        assert_eq!(
            effective_param_ownership(&looked, 0),
            OwnershipMode::Borrowed,
            "looked sig types={:?} ownership={:?}",
            looked.param_types,
            looked.param_ownership
        );

        codegen.in_impl_block = true;
        codegen.current_struct_name = Some("BuildFingerprint".into());
        if let Item::Impl { block, .. } = &program.items[0] {
            let generate_fn = block
                .functions
                .iter()
                .find(|f| f.name == "generate")
                .expect("generate fn");
            let let_stmt = generate_fn
                .body
                .iter()
                .find_map(|s| {
                    if let Statement::Let { value, .. } = s {
                        Some(value)
                    } else {
                        None
                    }
                })
                .expect("let in generate");
            if let Expression::Call {
                function,
                arguments: call_args,
                ..
            } = let_stmt
            {
                if let Expression::Identifier { name, .. } = function {
                    assert_eq!(name, "Self::collect_wj_files");
                } else {
                    panic!("expected Self::collect_wj_files identifier, got {function:?}");
                }
                let call = codegen.generate_expression(let_stmt);
                assert!(
                    !call.contains("source_dir.to_string()"),
                    "Self:: static call codegen must not to_string. Got:\n{call}"
                );
                assert!(
                    !call.contains(".clone()"),
                    "Self:: static call must not clone borrowed string arg. Got:\n{call}"
                );
                assert_eq!(call_args.len(), 1);
            } else {
                panic!("expected Call for collect_wj_files, got {let_stmt:?}");
            }
        }

        let rs = codegen.generate_program(&program, &analyzed);
        assert!(
            !rs.contains("source_dir.to_string()"),
            "library-style codegen must not to_string borrowed static string arg. Got:\n{rs}"
        );
        assert!(
            !rs.contains("hash_files(files.clone())"),
            "library-style codegen must not clone borrowed Vec arg. Got:\n{rs}"
        );
    }

    #[test]
    fn multipass_build_project_ext_static_self_borrows() {
        use crate::compiler::build_project_ext;
        use crate::CompilationTarget;
        use std::fs;
        use tempfile::TempDir;

        let source = r#"
impl BuildFingerprint {
    pub fn generate(source_dir: string) -> BuildFingerprint {
        let files = Self::collect_wj_files(source_dir)
        let hash = Self::hash_files(files)
        BuildFingerprint { source_hash: hash, build_timestamp: 0, source_files: files }
    }

    fn collect_wj_files(dir: string) -> Vec<string> {
        Vec::new()
    }

    fn hash_files(files: Vec<string>) -> u64 {
        0
    }
}
"#;
        let temp = TempDir::new().expect("tempdir");
        let src = temp.path().join("src");
        let build = temp.path().join("build");
        fs::create_dir_all(&src).expect("src");
        fs::create_dir_all(&build).expect("build");
        fs::write(src.join("build_fingerprint.wj"), source).expect("write wj");

        build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
            .expect("build_project_ext");

        let rs = fs::read_to_string(build.join("build_fingerprint.rs")).expect("read rs");
        assert!(
            !rs.contains("source_dir.to_string()"),
            "multipass must not to_string borrowed static string arg. Got:\n{rs}"
        );
        assert!(
            !rs.contains("hash_files(files.clone())"),
            "multipass must not clone borrowed Vec arg. Got:\n{rs}"
        );
    }

    #[test]
    fn converged_multi_arg_with_owned_copy_scalars_is_not_stale() {
        let sig = FunctionSignature {
            name: "QuestManager::update_objective_progress".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Reference(Box::new(Type::Custom("QuestId".into()))),
                Type::Custom("usize".into()),
                Type::Custom("u32".into()),
            ],
            param_ownership: vec![
                OwnershipMode::MutBorrowed,
                OwnershipMode::Borrowed,
                OwnershipMode::Owned,
                OwnershipMode::Owned,
            ],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
        };
        assert!(
            !has_stale_owned_non_copy_params(&sig),
            "Owned u32/usize must not mark converged QuestId borrow signature as stale"
        );
        assert!(
            !signature_is_declaration_stub_like(&sig),
            "converged multi-arg signature must not be stub-like"
        );
    }

    #[test]
    fn stale_engine_owned_non_copy_param_detected() {
        let sig = FunctionSignature {
            name: "QuestManager::is_quest_active".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Custom("QuestId".into()),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
            return_type: Some(Type::Custom("Bool".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
        };
        assert!(has_stale_owned_non_copy_params(&sig));
    }

    #[test]
    fn converged_owned_static_struct_param_not_stale() {
        let sig = FunctionSignature {
            name: "MannequinMesh::generate".into(),
            param_types: vec![Type::Custom("MannequinConfig".into())],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: Some(Type::Custom("MannequinMesh".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        };
        assert!(
            !has_stale_owned_non_copy_params(&sig),
            "owned consumption params must not be stub-like"
        );
        assert!(
            !signature_is_declaration_stub_like(&sig),
            "converged owned static method must resolve at call sites"
        );
    }

    #[test]
    fn resolve_pair_prefers_global_converged_quest_id_over_engine_stub() {
        let engine_stub = FunctionSignature {
            name: "QuestManager::is_quest_active".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Custom("QuestId".into()),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
            return_type: Some(Type::Custom("Bool".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
        };
        let converged = FunctionSignature {
            name: "QuestManager::is_quest_active".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Reference(Box::new(Type::Custom("QuestId".into()))),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("Bool".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
        };

        let mut local_reg = SignatureRegistry::new();
        local_reg.add_function("QuestManager::is_quest_active".into(), engine_stub);

        let mut global_reg = SignatureRegistry::new();
        global_reg.add_function("QuestManager::is_quest_active".into(), converged);

        let local = resolve_call_signature(
            &local_reg,
            "QuestManager::is_quest_active",
            Some("QuestManager"),
            1,
            &empty_aliases(),
            None,
        )
        .expect("local resolve");
        let global = resolve_call_signature(
            &global_reg,
            "QuestManager::is_quest_active",
            Some("QuestManager"),
            1,
            &empty_aliases(),
            None,
        )
        .expect("global resolve");

        let picked = pick_best_resolved_signature(Some(local), Some(global)).expect("pick");
        assert!(matches!(
            picked.sig.param_types[1],
            Type::Reference(_)
        ));
    }

    #[test]
    fn best_method_prefers_module_qualified_converged_over_stale_short_key() {
        let engine_stub = FunctionSignature {
            name: "QuestManager::is_quest_active".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Custom("QuestId".into()),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
            return_type: Some(Type::Custom("Bool".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
        };
        let converged = FunctionSignature {
            name: "quest::manager::QuestManager::is_quest_active".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Reference(Box::new(Type::Custom("QuestId".into()))),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("Bool".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
        };

        let mut reg = SignatureRegistry::new();
        reg.add_function("QuestManager::is_quest_active".into(), engine_stub);
        reg.add_function(
            "quest::manager::QuestManager::is_quest_active".into(),
            converged,
        );

        let resolved = resolve_call_signature(
            &reg,
            "QuestManager::is_quest_active",
            Some("QuestManager"),
            1,
            &empty_aliases(),
            None,
        )
        .expect("module-qualified converged should win");
        assert!(matches!(resolved.sig.param_types[1], Type::Reference(_)));
        assert_eq!(
            effective_param_ownership(&resolved.sig, 1),
            OwnershipMode::Borrowed
        );
    }

    #[test]
    fn prefer_converged_stale_engine_owned_quest_id_param() {
        let local = FunctionSignature {
            name: "QuestManager::is_quest_active".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Custom("QuestId".into()),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
            return_type: Some(Type::Custom("Bool".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
        };
        let global = FunctionSignature {
            name: "QuestManager::is_quest_active".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Reference(Box::new(Type::Custom("QuestId".into()))),
            ],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("Bool".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
        };
        assert!(prefer_converged_over_stub(&local, &global));

        let picked = pick_best_resolved_signature(
            Some(ResolvedSignature {
                sig: local,
                qualified_key: "QuestManager::is_quest_active".into(),
                resolution_method: ResolutionMethod::ReceiverQualified,
                has_collision: false,
            }),
            Some(ResolvedSignature {
                sig: global.clone(),
                qualified_key: "QuestManager::is_quest_active".into(),
                resolution_method: ResolutionMethod::ReceiverQualified,
                has_collision: false,
            }),
        );
        assert_eq!(
            picked.unwrap().sig.param_ownership[1],
            OwnershipMode::Borrowed
        );
    }
}
