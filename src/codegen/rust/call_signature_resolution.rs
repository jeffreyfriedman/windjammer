//! Unified call signature resolution.
//!
//! Every call expression in the compiler resolves its callee signature through
//! this single module. Resolution follows a strict precedence chain with NO
//! bare unqualified lookups (the root cause of the int-cast collision bug).

use std::collections::HashMap;

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::Type;

pub(crate) use super::signature_promotion::{
    best_method_signature_for_receiver, body_borrow_must_not_replace_owned_copy_formal,
    body_borrow_must_not_replace_owned_formal_stub, effective_user_arg_count,
    has_stale_owned_non_copy_params, param_type_is_owned_non_text, pick_best_resolved_signature,
    prefer_converged_over_stub, signature_is_declaration_stub_like,
};

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

/// Module-qualified user calls (`draw::draw_text`) disambiguate homonyms — keep auto-borrow.
/// Unqualified calls still respect global ownership-collision guards.
pub(crate) fn ownership_collision_blocks_autoborrow(callee_name: &str) -> bool {
    // Any `Module::method` or `Type::method` path is disambiguated — do not strip
    // auto-borrows. Bare `query` still blocks (http::query vs Connection::query).
    if callee_name.contains("::") {
        return false;
    }
    true
}

/// Whether registry metadata reports an ownership collision that should suppress
/// auto-borrow/`&` insertion for this call (uses `global_fallback` when layered).
pub(crate) fn registry_homonym_blocks_autoborrow(
    registry: &crate::analyzer::SignatureRegistry,
    callee_name: &str,
) -> bool {
    if !ownership_collision_blocks_autoborrow(callee_name) {
        return false;
    }
    let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
    if callee_name.contains("::") {
        registry.has_explicit_ownership_collision(callee_name)
            || registry.has_collision(callee_name)
    } else {
        registry.has_explicit_ownership_collision(simple) || registry.has_collision(simple)
    }
}

/// Strip auto-borrow/coercion artifacts when homonym ownership collision forbids them.
pub(crate) fn strip_collision_blocked_call_site_coercions(coerced: &mut String) {
    while coerced.starts_with('&') && !coerced.starts_with("&mut ") {
        *coerced = coerced[1..].to_string();
    }
    crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
    if let Some(stripped) = coerced.strip_suffix(".to_string()") {
        *coerced = stripped.to_string();
    }
}

/// `draw::draw_text`-style calls where the qualifier is a user module (not runtime std).
pub(crate) fn is_lowercase_user_module_qualified_call(callee_name: &str) -> bool {
    callee_name.rsplit_once("::").is_some_and(|(module, _)| {
        module
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
            && !crate::codegen::rust::stdlib_method_traits::is_runtime_std_module(module)
    })
}

/// Whether auto-borrow/`&mut` insertion must be skipped for this call.
///
/// Simple-name ownership collisions (e.g. two modules define `draw_text` with
/// different ownership) block auto-borrow even for module-qualified calls like
/// `hud_render::draw_text`. Type-only method-name collisions still honor
/// [`ownership_collision_blocks_autoborrow`] for disambiguated module paths.
pub(crate) fn has_ownership_collision_for_call(
    gen: &crate::codegen::rust::generator::CodeGenerator,
    func_name: &str,
) -> bool {
    if let Some(qualified) = gen.imported_runtime_qualified_callee(func_name) {
        return gen.has_explicit_ownership_collision_with_global(&qualified);
    }
    // `WalSegment::from_bytes` / `Vec::push` — the type prefix disambiguates homonyms;
    // do not block on simple-name method collisions (`from_bytes`, `new`, etc.).
    if is_type_qualified_associated_call(func_name) {
        return gen.has_explicit_ownership_collision_with_global(func_name);
    }
    let simple_name = func_name.rsplit("::").next().unwrap_or(func_name);
    if gen.has_explicit_ownership_collision_with_global(simple_name) {
        return true;
    }
    // Simple-name ownership/type collision (e.g. two modules' `draw_text`) must
    // block auto-borrow for `hud_render::draw_text` as well — check the simple
    // name with the simple-name autoborrow policy, not the module-qualified key
    // (lowercase module prefixes would otherwise skip the guard).
    if (gen.has_collision_with_global(simple_name)
        || gen.has_explicit_ownership_collision_with_global(simple_name))
        && ownership_collision_blocks_autoborrow(simple_name)
    {
        return true;
    }
    (gen.has_collision_with_global(func_name)
        || gen.has_explicit_ownership_collision_with_global(func_name))
        && ownership_collision_blocks_autoborrow(func_name)
}

/// Resolve a call signature from the registry.
///
/// Resolution precedence (each step tried only if previous returned `None`):
///
/// 1. **Exact key** — `registry.get_signature(func_name)`
/// 2. **Receiver-qualified** — `"{receiver_type}::{method}"` (and base-type variant)
/// 3. **Identifier-as-qualifier** — for `Foo::bar()` parsed as `Call(FieldAccess)`,
/// try `"{identifier}::{method}"` when identifier differs from receiver_type
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
        let converged =
            !signature_is_declaration_stub_like(sig) && !has_stale_owned_non_copy_params(sig);
        let affinity = caller_module
            .map(|caller| module_path_affinity(caller, key))
            .unwrap_or(0);
        let key_len = key.len();
        let replace =
            best.as_ref()
                .is_none_or(|(_, _, best_affinity, best_len, best_converged)| {
                    if converged && !best_converged {
                        return true;
                    }
                    if converged == *best_converged {
                        return affinity > *best_affinity
                            || (affinity == *best_affinity && key_len > *best_len);
                    }
                    false
                });
        if replace {
            best = Some((key.to_string(), sig.clone(), affinity, key_len, converged));
        }
    };

    // Prefer method-index: suffix is `::{Type}::{method}` or `::{method}` — index by
    // the trailing method segment so we never scan the full registry.
    let method_leaf = suffix
        .rsplit("::")
        .next()
        .unwrap_or(suffix.trim_start_matches(':'));
    if let Some(keys) = registry.method_keys_for(method_leaf) {
        for key in keys {
            if let Some(sig) = registry.get_signature(key) {
                consider(key, sig);
            }
        }
    } else {
        for (key, sig) in registry.all_signatures_for_suffix_search() {
            consider(key, sig);
        }
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
                    if qualifier
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_uppercase())
                    {
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
        if qualifier
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        {
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
            if qualifier
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
            {
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
        // Module-qualified lowercase calls (draw::draw_text): only accept
        // signatures from the SAME module. Falling back to an unrelated
        // module's homonym (rendering_api::draw_text) would apply wrong
        // ownership metadata.
        if let Some(pos) = func_name.rfind("::") {
            let module_qualifier = &func_name[..pos];
            if module_qualifier
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase())
            {
                let module_suffix = format!("{}::", module_qualifier);
                if let Some(sig) =
                    registry.find_signature_by_name_and_arg_count(method_part, arg_count)
                {
                    let qualified_key = registry
                        .signatures
                        .iter()
                        .find(|(_, v)| std::ptr::eq(*v, sig))
                        .map(|(k, _)| k.clone())
                        .unwrap_or_else(|| method_part.to_string());
                    if qualified_key.contains(&module_suffix) || qualified_key == func_name {
                        return Some(ResolvedSignature {
                            sig: sig.clone(),
                            qualified_key,
                            resolution_method: ResolutionMethod::ArgCountValidated,
                            has_collision: registry.has_collision(method_part),
                        });
                    }
                }
                // No match from the same module — don't fall back to a different module.
            } else if let Some(sig) =
                registry.find_signature_by_name_and_arg_count(method_part, arg_count)
            {
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

/// Single entry point for `ReceiverType::method` call-site signature resolution.
///
/// Combines `best_method_signature_for_receiver` on local and global registries,
/// applies `pick_best_resolved_signature`, and filters body-inferred borrows that
/// must not replace owned formal stubs (MannequinMesh::generate pattern).
pub fn resolve_method_for_call_site(
    local: &SignatureRegistry,
    global: Option<&SignatureRegistry>,
    receiver_type: &str,
    method: &str,
    arg_count: usize,
) -> Option<ResolvedSignature> {
    let to_resolved = |registry: &SignatureRegistry| -> Option<ResolvedSignature> {
        best_method_signature_for_receiver(registry, receiver_type, method, arg_count).map(
            |(qualified_key, sig)| {
                let collision_key = format!("{receiver_type}::{method}");
                ResolvedSignature {
                    sig,
                    qualified_key,
                    resolution_method: ResolutionMethod::ReceiverQualified,
                    has_collision: registry.has_collision(&collision_key),
                }
            },
        )
    };

    let local_resolved = to_resolved(local);
    let global_resolved = global.and_then(to_resolved);
    let local_resolved_for_refresh = local_resolved.clone();
    let global_resolved_for_refresh = global_resolved.clone();

    let (local_filtered, global_filtered) = match (&local_resolved, &global_resolved) {
        (Some(l), Some(g)) => {
            let mut l_out = local_resolved.clone();
            let mut g_out = global_resolved.clone();
            if body_borrow_must_not_replace_owned_formal_stub(&g.sig, &l.sig) {
                l_out = None;
            }
            if body_borrow_must_not_replace_owned_formal_stub(&l.sig, &g.sig) {
                g_out = None;
            }
            (l_out, g_out)
        }
        _ => (local_resolved, global_resolved),
    };

    let codegen_refresh_source = global_filtered
        .clone()
        .or_else(|| global_resolved_for_refresh.clone())
        .filter(|r| r.sig.emitted_rust_ref_params.is_some())
        .or_else(|| {
            local_filtered
                .clone()
                .or_else(|| local_resolved_for_refresh.clone())
                .filter(|r| r.sig.emitted_rust_ref_params.is_some())
        });

    pick_best_resolved_signature(local_filtered, global_filtered).map(|mut resolved| {
        let qualified = format!("{receiver_type}::{method}");
        if resolved.sig.emitted_rust_ref_params.is_none() {
            if let Some(alt) = &codegen_refresh_source {
                crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                    &mut resolved.sig,
                    &alt.sig,
                );
            } else if let Some(g) = global {
                // Defining-module refresh may only sit on the exact qualified key when
                // suffix/homonym resolution picked a stale body-converged borrow stub
                // (builder `Column` → `Table` owned column/row forward).
                if let Some(refreshed) = g.get_signature(&qualified) {
                    if refreshed.emitted_rust_ref_params.is_some() {
                        crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                            &mut resolved.sig,
                            refreshed,
                        );
                    }
                } else if let Some((_, refreshed)) =
                    best_method_signature_for_receiver(g, receiver_type, method, arg_count)
                {
                    // Bare `Type::method` may have been filtered; module-qualified
                    // defining-module meta still carries `emitted_rust_ref_params`.
                    if refreshed.emitted_rust_ref_params.is_some() {
                        crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                            &mut resolved.sig,
                            &refreshed,
                        );
                    }
                }
            }
        }
        if let Some(g) = global {
            apply_trait_owned_string_call_site_contracts(g, method, &mut resolved.sig);
        }
        resolved.sig = finalize_call_site_signature(resolved.sig);
        resolved
    })
}

/// When an impl method body converged `string` to `&str`, restore trait declaration owned
/// `String` call-site contracts from the global registry (port traits / E0053).
///
/// Only matches keys that are known trait definitions (registered via `trait_method_keys`).
/// This prevents unrelated impl methods with the same name from incorrectly colliding
/// (e.g. `Registry::register` must not affect `ComponentRegistry::register`).
/// True when any trait definition in `global` declares an owned plain `string` at `arg_index`
/// for instance method `method` (port-trait E0053 keep-owned at call sites).
pub(crate) fn global_trait_owned_plain_string_arg(
    global: &SignatureRegistry,
    method: &str,
    arg_index: usize,
) -> bool {
    let suffix = format!("::{method}");
    for (key, trait_sig) in &global.signatures {
        if !key.ends_with(&suffix) || !global.is_trait_method_key(key) {
            continue;
        }
        if !trait_sig.has_self_receiver {
            continue;
        }
        let pidx = trait_sig.arg_param_index(arg_index);
        if formal_is_plain_windjammer_string(trait_sig, pidx) {
            return true;
        }
    }
    false
}

pub(crate) fn apply_trait_owned_string_call_site_contracts(
    global: &SignatureRegistry,
    method: &str,
    sig: &mut FunctionSignature,
) {
    // Trait owned-string call-site contracts apply to instance methods only (port traits /
    // E0053). Static associated methods (`Squad::new`, `BuildFingerprint::collect_wj_files`)
    // keep body-converged `&str` — global declaration stubs must not downgrade them.
    if !sig.has_self_receiver {
        return;
    }
    let suffix = format!("::{method}");
    for (key, trait_sig) in &global.signatures {
        if key == &sig.name || !key.ends_with(&suffix) {
            continue;
        }
        // Only apply contracts from trait definitions, not arbitrary impl methods
        // with the same name. This prevents `Registry::register` (a regular impl)
        // from incorrectly upgrading `ComponentRegistry::register` to Owned.
        if !global.is_trait_method_key(key) {
            continue;
        }
        if trait_sig.has_self_receiver != sig.has_self_receiver {
            continue;
        }
        for idx in 0..sig.param_ownership.len() {
            if sig.has_self_receiver && idx == 0 {
                continue;
            }
            if !formal_is_plain_windjammer_string(trait_sig, idx) {
                continue;
            }
            // For trait definitions, the formal `string` type determines the contract
            // (Owned String). Body analysis may have overwritten param_ownership to
            // Borrowed, but the trait declaration is the source of truth.
            if !global.is_trait_method_key(key)
                && !matches!(
                    trait_sig.param_ownership.get(idx),
                    Some(OwnershipMode::Owned)
                )
            {
                continue;
            }
            let impl_converged_borrow = sig.param_types.get(idx).is_some_and(|ty| {
                crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
                    || (matches!(ty, Type::Reference(inner) if crate::codegen::rust::types::is_windjammer_text_type(inner))
                        && matches!(sig.param_ownership.get(idx), Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)))
            });
            let ownership_stale_borrow =
                sig.param_ownership.get(idx) == Some(&OwnershipMode::Borrowed);
            if !impl_converged_borrow && !ownership_stale_borrow {
                continue;
            }
            if let Some(t) = sig.param_types.get_mut(idx) {
                *t = Type::String;
            }
            if let Some(o) = sig.param_ownership.get_mut(idx) {
                *o = OwnershipMode::Owned;
            }
            if sig.formal_param_types.len() <= idx {
                sig.formal_param_types.resize(idx + 1, Type::String);
            } else {
                sig.formal_param_types[idx] = Type::String;
            }
            // Impl body may have preregistered `emitted_rust_ref_params[idx]=true` from
            // readonly `&str` convergence — trait owned `string` wins at call sites.
            let flag_len = sig.param_ownership.len();
            if sig.emitted_rust_ref_params.is_none() {
                sig.emitted_rust_ref_params = Some(vec![false; flag_len]);
            }
            if let Some(flags) = sig.emitted_rust_ref_params.as_mut() {
                if flags.len() < flag_len {
                    flags.resize(flag_len, false);
                }
                flags[idx] = false;
            }
        }
    }
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
                if best
                    .as_ref()
                    .is_none_or(|(best_len, _)| key_len > *best_len)
                {
                    best = Some((key_len, *own));
                }
            }
        }
    }
    best.map(|(_, own)| own)
}

/// Whether a formal type should honor a body-converged borrow.
pub(crate) fn formal_type_honors_converged_borrow(formal_ty: &Type) -> bool {
    match formal_ty {
        Type::Parameterized(base, _) => {
            crate::type_classification::is_stdlib_wrapper_type_base(base)
        }
        Type::String => true,
        Type::Custom(name) if name == "string" => true,
        Type::Custom(name)
            if crate::codegen::rust::type_analysis_pure::is_known_copy_type(name) =>
        {
            false
        }
        Type::Custom(_) => true,
        _ => !crate::codegen::rust::type_analysis_pure::is_copy_type(formal_ty),
    }
}

pub(crate) fn formal_is_plain_windjammer_string(sig: &FunctionSignature, param_idx: usize) -> bool {
    crate::ir::formal_predicates::formal_is_plain_windjammer_string(sig, param_idx)
}

/// Like [`formal_is_plain_windjammer_string`] but for a call-site argument index (accounts for `self`).
///
/// Method metadata sometimes stores `formal_param_types` without a `self` slot while
/// `arg_param_index` still offsets by `self_receiver_slot_count()` — check both layouts.
pub(crate) fn formal_is_plain_windjammer_string_for_call_arg(
    sig: &FunctionSignature,
    arg_index: usize,
) -> bool {
    crate::ir::formal_predicates::formal_is_plain_windjammer_string_for_call_arg(sig, arg_index)
}

/// Determine effective ownership for a parameter at a call site.
///
/// Resolution precedence:
/// 1. Static impl text borrows (body-converged `&str`)
/// 2. Trait instance owned string contracts
/// 3. Explicit `Reference`/`MutableReference` in param_types
/// 4. Body-converged text borrows
/// 5. Plain windjammer string formals
/// 6. Owned non-text struct formals
/// 7. Stored param_ownership fallback
pub fn effective_param_ownership(sig: &FunctionSignature, param_idx: usize) -> OwnershipMode {
    if crate::codegen::rust::call_site_borrow::plain_string_formal_passes_owned_at_call_site(
        sig, param_idx,
    ) {
        return OwnershipMode::Owned;
    }

    // Emitted Rust formal is owned (`deps: AppDeps` / `mut deps: AppDeps`) — call sites
    // must pass by value even when analyzer still marks MutBorrowed (Copy aggregates after
    // `.len()` + field method; reverse_journal_entry / codegen_mut_owned_param_moved).
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx) {
        return OwnershipMode::Owned;
    }

    if let Some(converged_ty) = sig.param_types.get(param_idx) {
        if crate::codegen::rust::string_utilities::param_is_rust_str_ref(converged_ty) {
            let trait_owned_string = sig.has_self_receiver
                && param_idx > 0
                && formal_is_plain_windjammer_string(sig, param_idx)
                && is_type_qualified_associated_call(&sig.name)
                && sig
                    .formal_param_type(param_idx)
                    .is_some_and(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)))
                && matches!(
                    sig.param_ownership.get(param_idx),
                    Some(OwnershipMode::Owned)
                );
            if !trait_owned_string {
                return OwnershipMode::Borrowed;
            }
        }
        match converged_ty {
            Type::Reference(inner)
                if !crate::codegen::rust::types::is_windjammer_text_type(inner) =>
            {
                if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                ) {
                    return OwnershipMode::Owned;
                }
                if sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx))
                    .copied()
                    == Some(false)
                {
                    return OwnershipMode::Owned;
                }
                // Stale `Reference(T)` when codegen emitted bare owned formal (`other: Lsn`).
                if let Some(formal) = sig.formal_param_type(param_idx) {
                    if !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                        && formal == inner.as_ref()
                    {
                        if matches!(
                            sig.param_ownership.get(param_idx),
                            Some(OwnershipMode::Owned)
                        ) || (crate::codegen::rust::type_analysis_pure::is_copy_type(formal)
                            || matches!(
                                formal,
                                Type::Custom(name)
                                    if crate::type_classification::is_known_copy_aggregate(name)
                            ))
                            && !crate::type_classification::is_copy_pass_by_value_formal(formal)
                        {
                            return OwnershipMode::Owned;
                        }
                    }
                }
                return OwnershipMode::Borrowed;
            }
            Type::MutableReference(inner)
                if !crate::codegen::rust::types::is_windjammer_text_type(inner) =>
            {
                if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                ) {
                    return OwnershipMode::Owned;
                }
                // Stale analyzer MutableReference on Copy aggregates (AppDeps) that emit as
                // owned `mut deps: AppDeps` — call sites must pass by value.
                // Do NOT treat bare non-Copy Custom (`Grid`) as owned: source formals stay
                // `Grid` while converged `param_types` is `MutableReference` + MutBorrowed,
                // and call sites must emit `&mut self.grid` (cross-file passthrough).
                if let Some(formal) = sig.formal_param_type(param_idx) {
                    let bare = match formal {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    let is_copy_aggregate =
                        crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
                            && !crate::type_classification::is_copy_pass_by_value_formal(bare);
                    if is_copy_aggregate
                        && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                        && !matches!(
                            sig.param_ownership.get(param_idx),
                            Some(OwnershipMode::MutBorrowed)
                        )
                    {
                        return OwnershipMode::Owned;
                    }
                }
                return OwnershipMode::MutBorrowed;
            }
            _ => {}
        }
    }

    // Rule 0: formal_params trump stale converged params (text types only).
    // Non-text Reference/MutableReference convergence is authoritative above.
    if let Some(formal_ty) = sig.formal_param_type(param_idx) {
        if !matches!(formal_ty, Type::Reference(_) | Type::MutableReference(_)) {
            if let Some(converged_ty) = sig.param_types.get(param_idx) {
                if matches!(converged_ty, Type::Reference(_) | Type::MutableReference(_)) {
                    if crate::codegen::rust::types::is_windjammer_text_type(formal_ty) {
                        if crate::codegen::rust::call_site_borrow::plain_string_formal_passes_owned_at_call_site(
                            sig, param_idx,
                        ) {
                            return OwnershipMode::Owned;
                        }
                        return sig
                            .param_ownership
                            .get(param_idx)
                            .copied()
                            .unwrap_or(OwnershipMode::Owned);
                    }
                }
            }
        }
    }

    if static_impl_text_borrows_at_call_site(sig, param_idx) {
        return OwnershipMode::Borrowed;
    }

    // Instance methods: owned String for plain `string` formals unless
    // the formal type itself is a reference. Body-converged `&str` and stale
    // Borrowed metadata must not downgrade call sites — generated Rust defs
    // use owned `String` for trait port methods (E0053 / E0308).
    if sig.has_self_receiver
        && param_idx > 0
        && formal_is_plain_windjammer_string(sig, param_idx)
        && is_type_qualified_associated_call(&sig.name)
        && sig
            .formal_param_type(param_idx)
            .is_some_and(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)))
    {
        return OwnershipMode::Owned;
    }

    // Converged &str or Reference(text) in param_types with matching ownership.
    if sig
        .param_types
        .get(param_idx)
        .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref)
        && !crate::codegen::rust::call_site_borrow::plain_string_formal_passes_owned_at_call_site(
            sig, param_idx,
        )
    {
        return OwnershipMode::Borrowed;
    }

    if let Some(ty) = sig.param_types.get(param_idx) {
        let ownership_borrowed = matches!(
            sig.param_ownership.get(param_idx),
            Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        );
        if ownership_borrowed {
            if crate::codegen::rust::string_utilities::param_is_rust_str_ref(ty)
                && !crate::codegen::rust::call_site_borrow::plain_string_formal_passes_owned_at_call_site(
                    sig, param_idx,
                )
            {
                return OwnershipMode::Borrowed;
            }
            if matches!(ty, Type::Reference(inner) if crate::codegen::rust::types::is_windjammer_text_type(inner))
                && !crate::codegen::rust::call_site_borrow::plain_string_formal_passes_owned_at_call_site(
                    sig, param_idx,
                )
            {
                return OwnershipMode::Borrowed;
            }
        }
        match ty {
            Type::Reference(inner) => {
                if sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx))
                    .copied()
                    == Some(false)
                {
                    return OwnershipMode::Owned;
                }
                if let Some(formal) = sig.formal_param_type(param_idx) {
                    if !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                        && formal == inner.as_ref()
                        && matches!(
                            sig.param_ownership.get(param_idx),
                            Some(OwnershipMode::Owned)
                        )
                    {
                        return OwnershipMode::Owned;
                    }
                }
                return OwnershipMode::Borrowed;
            }
            Type::MutableReference(_) => return OwnershipMode::MutBorrowed,
            _ => {}
        }
    }

    // Plain `string` formals: honor body-inferred borrow only after codegen confirms `&str`.
    if formal_is_plain_windjammer_string(sig, param_idx) {
        if sig
            .emitted_rust_ref_params
            .as_ref()
            .is_some_and(|flags| flags.get(param_idx).copied() == Some(false))
        {
            return OwnershipMode::Owned;
        }
        if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
            sig, param_idx,
        ) {
            return OwnershipMode::Borrowed;
        }
        if sig.emitted_rust_ref_params.is_none() {
            return OwnershipMode::Owned;
        }
        if matches!(
            sig.param_ownership.get(param_idx),
            Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        ) {
            return sig.param_ownership[param_idx];
        }
        return OwnershipMode::Owned;
    }

    // Owned string contract: param_types is String or Reference(text) with Owned ownership.
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Owned)
    ) && sig.param_types.get(param_idx).is_some_and(|t| {
        matches!(t, Type::String)
            || matches!(t, Type::Reference(inner) if crate::codegen::rust::types::is_windjammer_text_type(inner))
    }) {
        return OwnershipMode::Owned;
    }

    // Bare owned non-text formal: type-qualified methods pass by value.
    if param_type_is_owned_non_text(sig, param_idx)
        && sig
            .param_types
            .get(param_idx)
            .is_some_and(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)))
    {
        // Body-converged borrow on bare non-Copy formals (Map keys, mutating lookups)
        // must beat the default type-qualified owned pass-by-value rule — even when
        // param_types stayed bare `Custom(T)` without a Reference wrap (multipass engine
        // metadata promotion: QuestManager::is_quest_active id: Borrowed QuestId).
        if let Some(own) = sig.param_ownership.get(param_idx) {
            if matches!(own, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed) {
                if let Some(formal_ty) = sig.formal_param_type(param_idx) {
                    if formal_type_honors_converged_borrow(formal_ty) {
                        if matches!(own, OwnershipMode::MutBorrowed)
                            && crate::codegen::rust::type_analysis_pure::is_copy_type(formal_ty)
                        {
                            // Copy scalars and Copy aggregates that emit `&mut T`
                            // (`player: &mut PlayerState`) need MutBorrowed at call sites.
                            // Only demote to owned when codegen recorded an owned formal
                            // (AppDeps / regression-060) via emitted_owned_arg_contract.
                            if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                sig, param_idx,
                            ) {
                                return OwnershipMode::Owned;
                            }
                            return OwnershipMode::MutBorrowed;
                        }
                        if matches!(own, OwnershipMode::Borrowed) {
                            return OwnershipMode::Borrowed;
                        }
                    }
                }
            }
        }
        if is_type_qualified_associated_call(&sig.name) {
            return OwnershipMode::Owned;
        }
        if let Some(own) = sig.param_ownership.get(param_idx) {
            if matches!(own, OwnershipMode::MutBorrowed) {
                // Emitted owned formal wins over stale MutBorrowed (Copy aggregates
                // that stay `mut deps: AppDeps`). When MutBorrowed is genuine
                // (`player: &mut PlayerState`), emitted_owned_arg_contract is false.
                if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                ) {
                    return OwnershipMode::Owned;
                }
                return OwnershipMode::MutBorrowed;
            }
            if matches!(own, OwnershipMode::Borrowed) {
                if let Some(formal_ty) = sig.formal_param_type(param_idx) {
                    if formal_type_honors_converged_borrow(formal_ty) {
                        return *own;
                    }
                }
            }
        }
        return OwnershipMode::Owned;
    }

    sig.param_ownership
        .get(param_idx)
        .copied()
        .unwrap_or(OwnershipMode::Owned)
}

/// `station_builder::set_if`, not `Vec3::new` or bare `helper`.
pub(crate) fn is_external_module_qualified_call(func_name: &str) -> bool {
    func_name.contains("::") && func_name.chars().next().is_some_and(|c| c.is_lowercase())
}

/// Whether registry refresh for this callee must not consult bare method-name keys.
///
/// Type-qualified associated calls (`ServerResponse::error`) and imported runtime-std
/// module calls (`csv::write`, `strings::join`) are disambiguated by their qualifier.
/// Bare homonyms in the same file (`pub fn write` forwarding to `csv.write`) must not
/// poison the qualified callee's borrow contract.
pub(crate) fn qualified_callee_skips_bare_homonym_lookup(callee_name: &str) -> bool {
    if is_type_qualified_associated_call(callee_name) {
        return true;
    }
    callee_name.rsplit_once("::").is_some_and(|(module, _)| {
        module
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
            && crate::codegen::rust::stdlib_method_traits::is_runtime_std_module(module)
    })
}

pub fn effective_param_ownership_for_arg(
    sig: &FunctionSignature,
    arg_index: usize,
) -> OwnershipMode {
    let idx = sig.arg_param_index(arg_index);
    effective_param_ownership(sig, idx)
}

/// Whether a formal type is `*mut T` (FFI out-param / mutable raw pointer).
pub fn param_type_is_mutable_raw_pointer(ty: &Type) -> bool {
    matches!(ty, Type::RawPointer { mutable: true, .. })
}

/// Whether resolved callee metadata expects `&mut T` for a user argument index.
pub fn callee_user_arg_expects_mut_borrow(sig: &FunctionSignature, user_arg_index: usize) -> bool {
    let pidx = sig.arg_param_index(user_arg_index);
    if sig
        .param_types
        .get(pidx)
        .or_else(|| sig.formal_param_type(pidx))
        .is_some_and(param_type_is_mutable_raw_pointer)
    {
        return true;
    }
    if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
        return false;
    }
    // Copy aggregates that emit owned formals (`mut deps: AppDeps`) must not ask for
    // `&mut` at call sites. Copy aggregates that emit `&mut T` (`player: &mut PlayerState`)
    // keep MutBorrowed / MutableReference metadata — still ask for `&mut`.
    if let Some(formal) = sig.formal_param_type(pidx) {
        let bare = match formal {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        let copy_aggregate = crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
            && !crate::type_classification::is_copy_pass_by_value_formal(bare)
            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_));
        if copy_aggregate
            && !matches!(
                sig.param_ownership.get(pidx),
                Some(OwnershipMode::MutBorrowed)
            )
            && !matches!(sig.param_types.get(pidx), Some(Type::MutableReference(_)))
        {
            return false;
        }
    }
    matches!(sig.param_types.get(pidx), Some(Type::MutableReference(_)))
        || matches!(
            effective_param_ownership_for_arg(sig, user_arg_index),
            OwnershipMode::MutBorrowed,
        )
}

/// Static impl methods borrow at call sites when body analysis converged the param
/// to `&str` in param_types or Borrowed text in param_ownership.
pub(crate) fn static_impl_text_borrows_at_call_site(
    sig: &FunctionSignature,
    param_idx: usize,
) -> bool {
    if !is_type_qualified_associated_call(&sig.name) || sig.has_self_receiver {
        return false;
    }
    if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
        sig, param_idx,
    ) && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
        sig, param_idx,
    ) {
        return false;
    }
    // Body-converged &str — only when codegen/analyzer confirmed shared-ref emission.
    // Stale `Reference(str)` on owned plain-WJ static factories must not force call-site borrow.
    if sig
        .param_types
        .get(param_idx)
        .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref)
        && crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
            sig, param_idx,
        )
    {
        return true;
    }
    // Body-inferred Borrowed on text type
    matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
    ) && sig
        .param_types
        .get(param_idx)
        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
}

/// Like [`effective_param_ownership_for_arg`] but for method calls.
pub fn effective_param_ownership_for_method_arg(
    sig: &FunctionSignature,
    arg_index: usize,
    _receiver_type: Option<&str>,
) -> OwnershipMode {
    let idx = sig.arg_param_index(arg_index);
    effective_param_ownership(sig, idx)
}

/// E0053: plain `string` trait/item formals are owned `String` at call sites even when body
/// analysis converged `param_types` to `Reference(str)` and/or stale `param_ownership`.
pub fn normalize_owned_string_formal_for_call_site(sig: &mut FunctionSignature) {
    for idx in 0..sig.param_ownership.len() {
        if sig.has_self_receiver && idx == 0 {
            continue;
        }

        if static_impl_text_borrows_at_call_site(sig, idx) {
            continue;
        }

        // Codegen refresh recorded an emitted shared-ref formal — keep converged borrow.
        if sig
            .emitted_rust_ref_params
            .as_ref()
            .is_some_and(|flags| flags.get(idx).copied().unwrap_or(false))
        {
            continue;
        }

        // Instance impl/type methods with body-inferred borrow: keep &str (skip upgrade).
        if is_type_qualified_associated_call(&sig.name)
            && sig
                .param_types
                .get(idx)
                .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref)
            && matches!(
                sig.param_ownership.get(idx),
                Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
            )
            && !formal_is_plain_windjammer_string(sig, idx)
        {
            continue;
        }
        if is_type_qualified_associated_call(&sig.name)
            && sig
                .param_types
                .get(idx)
                .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref)
            && matches!(sig.param_ownership.get(idx), Some(OwnershipMode::Owned))
        {
            continue;
        }

        // Body-inferred borrow on plain `string` formals stays `&str` at call sites
        // (read-only impl helpers, passthrough wrappers). Trait impls force `Owned` in
        // analyzer merge when the trait item declares plain `string` (E0053).
        if formal_is_plain_windjammer_string(sig, idx)
            && matches!(
                sig.param_ownership.get(idx),
                Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
            )
        {
            continue;
        }

        let formal_plain_string = formal_is_plain_windjammer_string(sig, idx);

        let owned_string_contract = formal_plain_string
            || matches!(sig.param_ownership.get(idx), Some(OwnershipMode::Owned))
                && sig.param_types.get(idx).is_some_and(|t| {
                    matches!(t, Type::String)
                        || matches!(
                            t,
                            Type::Reference(inner)
                                if crate::codegen::rust::types::is_windjammer_text_type(inner)
                        )
                });

        if !owned_string_contract {
            continue;
        }

        if let Some(slot) = sig.param_ownership.get_mut(idx) {
            *slot = OwnershipMode::Owned;
        }
        if let Some(t) = sig.param_types.get_mut(idx) {
            if matches!(
                t,
                Type::Reference(inner)
                    if crate::codegen::rust::types::is_windjammer_text_type(inner)
            ) {
                *t = Type::String;
            }
        }
        if formal_plain_string {
            continue;
        }
        if sig.formal_param_types.len() <= idx {
            sig.formal_param_types.resize(idx + 1, Type::String);
        } else if crate::codegen::rust::types::is_windjammer_text_type(&sig.formal_param_types[idx])
        {
            sig.formal_param_types[idx] = Type::String;
        }
    }
}

pub fn finalize_call_site_signature(mut sig: FunctionSignature) -> FunctionSignature {
    if !sig.has_self_receiver && sig.has_self_receiver_slot() {
        sig.has_self_receiver = true;
    }
    align_sig_with_emitted_rust_ref_params(&mut sig);
    normalize_owned_string_formal_for_call_site(&mut sig);
    sig
}

/// When codegen refresh recorded emitted Rust formals, align stale converged
/// `Reference(T)` metadata with the actual generated definition.
fn align_sig_with_emitted_rust_ref_params(sig: &mut FunctionSignature) {
    let Some(ref flags) = sig.emitted_rust_ref_params else {
        return;
    };
    for idx in 0..flags.len() {
        if sig.has_self_receiver && idx == 0 {
            continue;
        }
        if flags.get(idx).copied() != Some(false) {
            continue;
        }
        // Body-converged / promoted borrow metadata beats stale engine emitted-owned flags,
        // except when the WJ formal is bare Custom (including Copy aggregates like Lsn) —
        // those always emit owned formals. Non-Copy demotions to `&T` set
        // `emitted_rust_ref_params[idx] == true` instead of false.
        if matches!(
            sig.param_ownership.get(idx),
            Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        ) {
            let bare_custom_owned_emit = sig.formal_param_type(idx).is_some_and(|t| {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                matches!(bare, Type::Custom(_))
                    && !crate::codegen::rust::types::is_windjammer_text_type(bare)
            }) || sig.formal_param_types.get(idx).is_some_and(|t| {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                matches!(bare, Type::Custom(_))
                    && !crate::codegen::rust::types::is_windjammer_text_type(bare)
            });
            let copy_aggregate_owned_emit = sig.formal_param_type(idx).is_some_and(|t| {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
                    && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                    && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
            });
            if !bare_custom_owned_emit && !copy_aggregate_owned_emit {
                continue;
            }
        }
        let owned_formal = sig.formal_param_type(idx).map(|t| match t {
            Type::Reference(inner) | Type::MutableReference(inner) => *inner.clone(),
            other => other.clone(),
        });
        let Some(formal) = owned_formal else {
            // Sparse metadata (library multipass export): emitted-owned still peels stale
            // body-converged `Reference(T)` wraps on the param slot.
            if let Some(t) = sig.param_types.get(idx) {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => {
                        inner.as_ref().clone()
                    }
                    other => other.clone(),
                };
                if idx < sig.param_types.len() {
                    sig.param_types[idx] = bare;
                }
                if idx < sig.param_ownership.len() {
                    sig.param_ownership[idx] = OwnershipMode::Owned;
                }
            }
            continue;
        };
        if idx < sig.param_types.len() {
            sig.param_types[idx] = formal.clone();
        }
        if idx < sig.param_ownership.len() {
            sig.param_ownership[idx] = OwnershipMode::Owned;
        }
    }
}

/// `MannequinMesh::generate`, `Vec::push` — not `foo::bar` module paths.
pub fn is_type_qualified_associated_call(func_name: &str) -> bool {
    let Some((type_part, _method)) = func_name.rsplit_once("::") else {
        return false;
    };
    type_part
        .rsplit("::")
        .next()
        .is_some_and(|leaf| leaf.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
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
            formal_param_types: vec![],

            param_ownership: vec![OwnershipMode::Owned; param_count + if has_self { 1 } else { 0 }],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: has_self,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    fn make_sig_with_types(name: &str, types: Vec<Type>, has_self: bool) -> FunctionSignature {
        let ownership_len = types.len() + if has_self { 1 } else { 0 };
        FunctionSignature {
            name: name.to_string(),
            param_types: types,
            formal_param_types: vec![],

            param_ownership: vec![OwnershipMode::Owned; ownership_len],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: has_self,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
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

        let result =
            resolve_call_signature(&reg, "new", Some("Emitter"), 2, &empty_aliases(), None);
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
    fn apply_trait_owned_string_skips_static_associated_methods() {
        let mut global = SignatureRegistry::new();
        global.add_function(
            "Squad::new".into(),
            FunctionSignature {
                name: "new".into(),
                param_types: vec![Type::String, Type::String],
                formal_param_types: vec![Type::String, Type::String],
                param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Owned],
                return_type: Some(Type::Custom("Squad".into())),
                return_ownership: OwnershipMode::Owned,
                has_self_receiver: false,
                is_extern: false,
                emitted_rust_ref_params: None,
                string_ref_string_formal_params: None,
                field_extract_params: None,
                forwarding_borrow_params: None,
            },
        );

        let mut sig = FunctionSignature {
            name: "new".into(),
            param_types: vec![
                Type::Reference(Box::new(Type::Custom("str".into()))),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![Type::String, Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("Squad".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };

        apply_trait_owned_string_call_site_contracts(&global, "new", &mut sig);
        assert!(
            sig.param_types
                .iter()
                .all(|t| { crate::codegen::rust::string_utilities::param_is_rust_str_ref(t) }),
            "static impl must keep converged &str despite global owned stub"
        );
    }

    #[test]
    fn normalize_preserves_body_converged_borrow_for_instance_methods() {
        let mut sig = FunctionSignature {
            name: "DbReportReader::report_lines".into(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![Type::Custom("Self".into()), Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Parameterized(
                "Vec".into(),
                vec![Type::Custom("ReportLine".into())],
            )),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        normalize_owned_string_formal_for_call_site(&mut sig);
        // Body analysis converged to &str + Borrowed → normalization preserves borrow.
        // Instance methods with body-converged &str should NOT upgrade to owned.
        assert_eq!(
            sig.param_types.get(1),
            Some(&Type::Reference(Box::new(Type::Custom("str".into())))),
            "body-converged &str must NOT be upgraded for instance methods"
        );
    }

    #[test]
    fn impl_method_str_ref_wins_over_stale_owned_metadata() {
        let sig = FunctionSignature {
            name: "Squad::new".into(),
            param_types: vec![
                Type::Reference(Box::new(Type::Custom("str".into()))),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![Type::String, Type::String],
            param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Owned],
            return_type: Some(Type::Custom("Squad".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        assert_eq!(
            effective_param_ownership_for_arg(&sig, 0),
            OwnershipMode::Borrowed,
            "converged &str param_types must borrow at call site even if param_ownership is stale Owned"
        );
        let mut normalized = sig.clone();
        normalize_owned_string_formal_for_call_site(&mut normalized);
        assert!(
            normalized
                .param_types
                .iter()
                .all(|t| { crate::codegen::rust::string_utilities::param_is_rust_str_ref(t) }),
            "normalize must not upgrade converged &str impl params to String"
        );
    }

    #[test]
    fn associated_plain_string_formal_owned_at_call_site_despite_body_borrow() {
        let sig = FunctionSignature {
            name: "EnvAccountReader::list_accounts".into(),
            param_types: vec![Type::Custom("Self".into()), Type::String],
            formal_param_types: vec![Type::Custom("Self".into()), Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Parameterized(
                "Vec".into(),
                vec![Type::Custom("Account".into())],
            )),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        assert_eq!(
            effective_param_ownership_for_arg(&sig, 0),
            OwnershipMode::Owned,
            "plain string trait formal must pass owned String at call site"
        );
    }

    #[test]
    fn callee_user_arg_expects_mut_borrow_for_mutable_raw_pointer_formal() {
        let sig = FunctionSignature {
            name: "get_value".into(),
            param_types: vec![Type::RawPointer {
                mutable: true,
                pointee: Box::new(Type::Float),
            }],
            formal_param_types: vec![Type::RawPointer {
                mutable: true,
                pointee: Box::new(Type::Float),
            }],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: true,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        assert!(
            callee_user_arg_expects_mut_borrow(&sig, 0),
            "*mut T extern formals must request &mut at call sites"
        );
        assert!(
            !callee_user_arg_expects_mut_borrow(
                &FunctionSignature {
                    param_types: vec![Type::RawPointer {
                        mutable: false,
                        pointee: Box::new(Type::Float),
                    }],
                    formal_param_types: vec![Type::RawPointer {
                        mutable: false,
                        pointee: Box::new(Type::Float),
                    }],
                    ..sig
                },
                0
            ),
            "*const T must not request &mut"
        );
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
    fn owned_string_formal_is_owned_at_call_site_despite_str_ref_param_types() {
        let sig = FunctionSignature {
            name: "SeedAccountReader::list_accounts".to_string(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Reference(Box::new(Type::Custom("str".into()))),
            ],
            formal_param_types: vec![Type::Custom("Self".into()), Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
            return_type: Some(Type::Parameterized(
                "Vec".into(),
                vec![Type::Custom("Account".into())],
            )),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        assert_eq!(
            effective_param_ownership(&sig, 1),
            OwnershipMode::Owned,
            "trait owned string formal passes String by value even when body converged param_types to &str"
        );
    }

    #[test]
    fn stale_borrowed_metadata_on_owned_struct_param_is_owned() {
        let sig = FunctionSignature {
            name: "svo64_convert::voxelgrid_to_svo64_flat".to_string(),
            param_types: vec![Type::Custom("VoxelGrid".to_string())],
            formal_param_types: vec![],

            param_ownership: vec![OwnershipMode::Borrowed],
            return_type: Some(Type::Parameterized(
                "Vec".to_string(),
                vec![Type::Custom("u32".to_string())],
            )),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
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
    fn stale_mut_borrowed_on_owned_struct_formal_passes_by_value() {
        let sig = FunctionSignature {
            name: "composition::handlers::create_journal_entry".to_string(),
            param_types: vec![Type::Custom("AppDeps".to_string())],
            formal_param_types: vec![Type::Custom("AppDeps".to_string())],
            param_ownership: vec![OwnershipMode::MutBorrowed],
            return_type: Some(Type::Custom("PostedJournalEntry".to_string())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        assert_eq!(
            effective_param_ownership(&sig, 0),
            OwnershipMode::Owned,
            "owned non-ref formals pass by value even when body analysis marked MutBorrowed"
        );
    }

    #[test]
    fn reference_wrapped_user_type_borrows_despite_stale_owned_metadata() {
        let sig = FunctionSignature {
            name: "MemoryEngine::get".to_string(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Reference(Box::new(Type::Custom("Key".into()))),
            ],
            formal_param_types: vec![Type::Custom("Self".into()), Type::Custom("Key".into())],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Owned],
            return_type: Some(Type::Custom("i64".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        assert_eq!(
            effective_param_ownership(&sig, 1),
            OwnershipMode::Borrowed,
            "body-converged Reference(Key) must borrow at call site even when param_ownership is stale Owned"
        );
    }

    #[test]
    fn reference_wrapped_struct_param_is_borrowed() {
        let sig = FunctionSignature {
            name: "QuestManager::update_objective_progress".to_string(),
            param_types: vec![Type::Reference(Box::new(Type::Custom(
                "QuestId".to_string(),
            )))],
            formal_param_types: vec![],

            param_ownership: vec![OwnershipMode::Borrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        assert_eq!(effective_param_ownership(&sig, 0), OwnershipMode::Borrowed,);
        assert!(
            !param_type_is_owned_non_text(&sig, 0),
            "Reference(QuestId) is not owned"
        );
    }

    #[test]
    fn reference_wrapped_vec_honors_borrow_despite_stale_owned_metadata() {
        let vec_ty = Type::Parameterized("Vec".into(), vec![Type::Custom("u8".into())]);
        let sig = FunctionSignature {
            name: "ComponentRegistry::add".to_string(),
            param_types: vec![
                Type::Custom("Self".into()),
                Type::Custom("i64".into()),
                Type::Custom("ComponentId".into()),
                Type::Reference(Box::new(vec_ty.clone())),
            ],
            formal_param_types: vec![
                Type::Custom("Self".into()),
                Type::Custom("i64".into()),
                Type::Custom("ComponentId".into()),
                vec_ty,
            ],
            param_ownership: vec![
                OwnershipMode::Borrowed,
                OwnershipMode::Owned,
                OwnershipMode::Owned,
                OwnershipMode::Owned,
            ],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        assert_eq!(
            effective_param_ownership(&sig, 3),
            OwnershipMode::Borrowed,
            "Reference(Vec) wrapper must win over stale Owned param_ownership"
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
            rs.contains("Self::hash_files(&files)")
                || rs.contains("Self::hash_files(files.as_ref())"),
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
    fn draw_text_borrows_owned_string_local() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;

        let source = r#"
struct Renderer {}
impl Renderer {
    fn draw_text(self, text: string) {
        println("{}", text)
    }
}
fn main() {
    let renderer = Renderer{}
    let message = "Hello".to_string()
    renderer.draw_text(message)
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        // Empty `Type{}` must parse as StructLiteral (not Identifier + empty Block).
        let renderer_let = program.items.iter().find_map(|item| {
            if let crate::parser::Item::Function { decl, .. } = item {
                if decl.name == "main" {
                    return decl.body.first();
                }
            }
            None
        });
        assert!(
            matches!(
                renderer_let,
                Some(crate::parser::Statement::Let {
                    value: crate::parser::Expression::StructLiteral { name, fields, .. },
                    ..
                }) if name == "Renderer" && fields.is_empty()
            ),
            "Renderer{{}} must be StructLiteral, got {renderer_let:?}"
        );
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let mut codegen = CodeGenerator::new(registry, CompilationTarget::Rust);
        let rs = codegen.generate_program(&program, &analyzed);
        assert!(
            rs.contains("draw_text(&message)"),
            "owned String local → &str formal must borrow. Got:\n{rs}"
        );
    }

    #[test]
    fn cross_file_walls_borrow_via_layered_registry() {
        use crate::analyzer::{Analyzer, OwnershipMode};
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;
        use std::sync::Arc;

        let file_a = r#"
struct AABB { min_x: f32, max_x: f32 }
fn check_collisions(walls: Vec<AABB>) -> bool {
    let mut i = 0
    while i < walls.len() {
        if walls[i].min_x > 0.0 { return true }
        i = i + 1
    }
    false
}
"#;
        let mut lexer_a = Lexer::new(file_a);
        let tokens_a = lexer_a.tokenize_with_locations();
        let mut parser_a = Parser::new(tokens_a);
        let program_a = parser_a.parse().unwrap();
        let mut analyzer_a = Analyzer::new();
        let (_, registry_a, _) = analyzer_a.analyze_program(&program_a).unwrap();
        assert_eq!(
            registry_a
                .get_signature("check_collisions")
                .unwrap()
                .param_ownership[0],
            OwnershipMode::Borrowed
        );

        let file_b = r#"
struct AABB { min_x: f32, max_x: f32 }
fn get_walls() -> Vec<AABB> { Vec::new() }
fn game_update() {
    let walls = get_walls()
    let result = check_collisions(walls)
}
"#;
        let mut lexer_b = Lexer::new(file_b);
        let tokens_b = lexer_b.tokenize_with_locations();
        let mut parser_b = Parser::new(tokens_b);
        let program_b = parser_b.parse().unwrap();
        let mut analyzer_b = Analyzer::new();
        let (analyzed, registry, _) = analyzer_b
            .analyze_program_with_global_signatures(&program_b, &registry_a)
            .unwrap();
        let found = registry
            .get_signature("check_collisions")
            .cloned()
            .expect("sig");
        assert!(crate::ir::signature_bridge::call_site_expects_shared_borrow(&found, 0));
        let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
        codegen.set_global_signature_registry(Arc::new(registry_a));
        let rs = codegen.generate_program(&program_b, &analyzed);
        assert!(
            rs.contains("check_collisions(&walls)"),
            "expected &walls. Got:\n{rs}"
        );
    }

    #[test]
    fn cross_file_dense_csr_borrow_via_layered_registry() {
        use crate::analyzer::{Analyzer, OwnershipMode};
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;
        use std::sync::Arc;

        let file_a = r#"
pub struct DenseCsr {
    out_offsets: Vec<u32>,
}
pub fn graph_bfs_run_dense(csr: DenseCsr, source: i64) -> i64 {
    (csr.out_offsets.len() as i64) + source
}
"#;
        let mut lexer_a = Lexer::new(file_a);
        let tokens_a = lexer_a.tokenize_with_locations();
        let mut parser_a = Parser::new(tokens_a);
        let program_a = parser_a.parse().unwrap();
        let mut analyzer_a = Analyzer::new();
        let (_, registry_a, _) = analyzer_a.analyze_program(&program_a).unwrap();
        assert_eq!(
            registry_a
                .get_signature("graph_bfs_run_dense")
                .unwrap()
                .param_ownership[0],
            OwnershipMode::Borrowed
        );
        assert!(
            crate::ir::signature_bridge::call_site_expects_shared_borrow(
                registry_a.get_signature("graph_bfs_run_dense").unwrap(),
                0
            )
        );

        let file_b = r#"
pub struct DenseCsr {
    out_offsets: Vec<u32>,
}
pub fn make_csr() -> DenseCsr {
    DenseCsr { out_offsets: Vec::new() }
}
pub fn run_bfs(source: i64) -> i64 {
    let csr = make_csr()
    graph_bfs_run_dense(csr, source)
}
"#;
        let mut lexer_b = Lexer::new(file_b);
        let tokens_b = lexer_b.tokenize_with_locations();
        let mut parser_b = Parser::new(tokens_b);
        let program_b = parser_b.parse().unwrap();
        let mut analyzer_b = Analyzer::new();
        let (analyzed, registry, _) = analyzer_b
            .analyze_program_with_global_signatures(&program_b, &registry_a)
            .unwrap();
        let found = registry
            .get_signature("graph_bfs_run_dense")
            .cloned()
            .expect("sig");
        assert!(
            crate::ir::signature_bridge::call_site_expects_shared_borrow(&found, 0),
            "merged sig must expect shared borrow"
        );
        let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
        codegen.set_global_signature_registry(Arc::new(registry_a));
        let rs = codegen.generate_program(&program_b, &analyzed);
        assert!(
            rs.contains("graph_bfs_run_dense(&csr"),
            "expected &csr. Got:\n{rs}"
        );
        assert!(
            !rs.contains("graph_bfs_run_dense(csr.clone()"),
            "must not clone into &DenseCsr. Got:\n{rs}"
        );
    }

    #[test]
    fn trait_impl_copy_camera_stays_owned() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;

        let source = r#"
struct CameraData {
    fov: f32
    near: f32
    far: f32
}
trait RenderPort {
    fn set_camera(camera: CameraData)
}
struct VoxelRenderer {
    active: bool
}
impl RenderPort for VoxelRenderer {
    fn set_camera(camera: CameraData) {
        self.active = true
    }
}
struct Editor {
    renderer: VoxelRenderer
}
impl Editor {
    pub fn update_camera(self) {
        let camera = CameraData { fov: 60.0, near: 0.1, far: 100.0 }
        self.renderer.set_camera(camera)
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
            !rs.contains("_camera: &mut CameraData") && !rs.contains("camera: &mut CameraData"),
            "CameraData must stay owned in trait impl. Got:\\n{rs}"
        );
        assert!(
            rs.contains("set_camera(camera)") && !rs.contains("set_camera(&mut camera)"),
            "call site must pass by value. Got:\\n{rs}"
        );
    }

    #[test]
    fn subprocess_spawn_codegen_auto_borrows_vec_args() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;

        let source = r#"
    use std::subprocess

    fn test_echo() {
        let args = vec!["hello".to_string()]
        subprocess::spawn("echo", args)
    }
    "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let resolved = crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
            &registry,
            "subprocess::spawn",
            registry.get_signature("subprocess::spawn"),
            1,
        );
        assert!(resolved, "registry should require borrow for spawn args");
        let mut generator =
            CodeGenerator::new_for_module(registry.clone(), CompilationTarget::Rust);
        assert!(
            generator.ir_cutover.call_sites,
            "call_sites cutover must be on"
        );
        let spawn_call = program
            .items
            .iter()
            .find_map(|item| {
                if let crate::parser::Item::Function { decl, .. } = item {
                    decl.body.iter().find_map(|stmt| {
                        if let crate::parser::Statement::Expression { expr, .. } = stmt {
                            if let crate::parser::Expression::Call { arguments, .. } = expr {
                                return arguments.get(1).map(|(_, a)| *a);
                            }
                        }
                        None
                    })
                } else {
                    None
                }
            })
            .expect("spawn call arg");
        let finished = generator.finish_runtime_std_call_arg(
            "subprocess::spawn",
            1,
            spawn_call,
            "args".to_string(),
            registry.get_signature("subprocess::spawn"),
            None,
        );
        assert_eq!(
            finished, "&args",
            "finish_runtime_std_call_arg should borrow"
        );
        let ir_coerced = generator.apply_ir_call_site_coercion(
            &registry,
            "subprocess::spawn",
            1,
            spawn_call,
            "args",
            None,
            None,
            Some(2),
        );
        assert_eq!(
            ir_coerced.as_deref(),
            Some("&args"),
            "IR call-site coercion should auto-borrow Vec arg"
        );
        let output = generator.generate_program(&program, &analyzed);
        assert!(
            output.contains("subprocess::spawn(\"echo\", &args)"),
            "expected runtime auto-borrow for Vec args, got:\n{output}"
        );
    }

    #[test]
    fn json_get_codegen_auto_borrows_owned_value() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;

        let source = r#"
use std::json

pub fn parse_field(line: string) -> string {
    match json::parse(line) {
        Ok(v) => {
            if let Some(f) = json::get(v, "field") {
                return ""
            }
            return ""
        }
        Err(_) => return ""
    }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let mut generator = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
        let output = generator.generate_program(&program, &analyzed);
        assert!(
            output.contains("json::get(&v,") || output.contains("json::get(& v,"),
            "expected json::get(&v, ...) got:\n{output}"
        );
    }

    #[test]
    fn hashmap_insert_owned_key_no_redundant_clone() {
        use crate::analyzer::Analyzer;
        use crate::codegen::rust::CodeGenerator;
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::CompilationTarget;

        let source = r#"
    use std::collections::HashMap

    pub fn test(mut map: HashMap<string, int>, key: string) -> bool {
        map.insert(key, 42);
        return map.contains_key("test")
    }
    "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let mut generator = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
        let output = generator.generate_program(&program, &analyzed);
        assert!(
            (output.contains("map.insert(key, 42)")
                || output.contains("map.insert(key, 42_i32)")
                || output.contains("map.insert(key, 42_i64)")
                || output.contains("map.insert(key.to_string(), 42_i64)")
                || output.contains("map.insert(key.to_string(), 42_i32)"))
                && !output.contains("map.insert(&key")
                && !output.contains(".to_string().clone()"),
            "insert should not add redundant clone. Generated:\n{output}"
        );
    }

    #[test]
    fn strip_collision_blocked_removes_full_to_string_suffix() {
        let mut s = r#""recover".to_string()"#.to_string();
        strip_collision_blocked_call_site_coercions(&mut s);
        assert_eq!(s, r#""recover""#);
    }
}
