//! Unified call signature resolution.
//!
//! Every call expression in the compiler resolves its callee signature through
//! this single module. Resolution follows a strict precedence chain with NO
//! bare unqualified lookups (the root cause of the int-cast collision bug).

use std::collections::HashMap;

use crate::analyzer::{FunctionSignature, SignatureRegistry};

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
pub fn resolve_call_signature(
    registry: &SignatureRegistry,
    func_name: &str,
    receiver_type: Option<&str>,
    arg_count: usize,
    module_aliases: &HashMap<String, String>,
) -> Option<ResolvedSignature> {
    // Step 1: Exact key match (handles already-qualified names like "Vec::push").
    if let Some(sig) = registry.get_signature(func_name) {
        if validate_arg_count(sig, arg_count) {
            return Some(ResolvedSignature {
                sig: sig.clone(),
                qualified_key: func_name.to_string(),
                resolution_method: ResolutionMethod::ExactQualified,
                has_collision: registry.has_collision(func_name),
            });
        }
        // Key exists but arg count is wrong. If there's no collision, the
        // exact key is the only registration — don't fall through to suffix
        // matching which could pick a different type's method.
        // If there IS a collision, fall through: the right variant may be
        // registered under a longer-qualified key.
        if func_name.contains("::") && !registry.has_collision(func_name) {
            return None;
        }
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

    // Step 6: Arg-count-validated suffix match (last resort).
    // Uses find_signature_by_name_and_arg_count which searches all `::method` entries
    // but validates arg count. Only matches when the func_name already contains `::`
    // (type or module qualifier) — bare names like `update` should not match
    // `Component::update` because methods and free functions have different semantics.
    if func_name.contains("::") {
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

    None
}

/// Try receiver-type-qualified lookup with base-type stripping.
fn try_receiver_qualified(
    registry: &SignatureRegistry,
    method: &str,
    receiver_type: &str,
    arg_count: usize,
) -> Option<ResolvedSignature> {
    let base = receiver_type.split('<').next().unwrap_or(receiver_type);

    let qualified = format!("{}::{}", base, method);
    if let Some(sig) = registry.get_signature(&qualified) {
        if validate_arg_count(sig, arg_count) {
            let has_collision = registry.has_collision(&qualified);
            return Some(ResolvedSignature {
                sig: sig.clone(),
                qualified_key: qualified,
                resolution_method: ResolutionMethod::ReceiverQualified,
                has_collision,
            });
        }
    }

    if base != receiver_type {
        let full_qualified = format!("{}::{}", receiver_type, method);
        if let Some(sig) = registry.get_signature(&full_qualified) {
            if validate_arg_count(sig, arg_count) {
                let has_collision = registry.has_collision(&full_qualified);
                return Some(ResolvedSignature {
                    sig: sig.clone(),
                    qualified_key: full_qualified,
                    resolution_method: ResolutionMethod::ReceiverQualified,
                    has_collision,
                });
            }
        }
    }

    None
}

/// Validate that a signature's expected argument count matches the call site.
fn validate_arg_count(sig: &FunctionSignature, call_arg_count: usize) -> bool {
    let expected = if sig.has_self_receiver {
        sig.param_ownership.len().saturating_sub(1)
    } else {
        sig.param_ownership.len()
    };
    expected == call_arg_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::OwnershipMode;
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

    fn make_sig_with_types(
        name: &str,
        types: Vec<Type>,
        has_self: bool,
    ) -> FunctionSignature {
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

        let result = resolve_call_signature(&reg, "Vec::push", None, 1, &empty_aliases());
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
            resolve_call_signature(&reg, "new", Some("Emitter"), 2, &empty_aliases());
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
                vec![
                    Type::Custom("Vec3".into()),
                    Type::Custom("i32".into()),
                ],
                false,
            ),
        );

        // Looking up "new" bare with 2 args should NOT match Vec3::new (3 args)
        // and SHOULD match Emitter::new (2 args) via arg-count validation
        let result =
            resolve_call_signature(&reg, "Emitter::new", None, 2, &empty_aliases());
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.qualified_key, "Emitter::new");
        assert!(!r.sig.param_types.iter().any(|t| matches!(t, Type::Custom(n) if n == "f32")));
    }

    #[test]
    fn module_alias_resolution() {
        let mut reg = SignatureRegistry::new();
        reg.add_function("gpu_safe::load_shader".into(), make_sig("load_shader", 1, false));

        let mut aliases = HashMap::new();
        aliases.insert("gpu".into(), "gpu_safe".into());

        let result =
            resolve_call_signature(&reg, "gpu::load_shader", None, 1, &aliases);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.resolution_method, ResolutionMethod::ModuleAlias);
    }

    #[test]
    fn arg_count_mismatch_rejects() {
        let mut reg = SignatureRegistry::new();
        reg.add_function("Foo::new".into(), make_sig("new", 3, false));

        // Call with 2 args should NOT match a 3-param signature
        let result =
            resolve_call_signature(&reg, "Foo::new", None, 2, &empty_aliases());
        assert!(result.is_none());
    }

    #[test]
    fn collision_detected() {
        let mut reg = SignatureRegistry::new();
        reg.add_function(
            "Emitter::new".into(),
            make_sig_with_types("new", vec![Type::Custom("Vec3".into()), Type::Custom("i32".into())], false),
        );
        reg.add_function(
            "Emitter::new".into(),
            make_sig_with_types("new", vec![Type::Custom("f32".into()), Type::Custom("f32".into())], false),
        );

        let result =
            resolve_call_signature(&reg, "Emitter::new", None, 2, &empty_aliases());
        assert!(result.is_some());
        assert!(result.unwrap().has_collision);
    }

    #[test]
    fn progressive_qualified_match() {
        let mut reg = SignatureRegistry::new();
        reg.add_function("rendering::Camera::update".into(), make_sig("update", 1, true));

        let result = resolve_call_signature(
            &reg,
            "scene::rendering::Camera::update",
            None,
            1,
            &empty_aliases(),
        );
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.resolution_method, ResolutionMethod::ProgressiveQualified);
    }
}
