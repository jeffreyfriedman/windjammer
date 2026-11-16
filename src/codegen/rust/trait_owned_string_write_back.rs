//! Trait AST `string` (E0053) written back onto `impl Trait for Type` method signatures of a registry.

use crate::analyzer::{FunctionSignature, SignatureRegistry};
use std::collections::HashMap;

/// Write trait AST `string` (E0053) onto `impl Trait for Type` method signatures
/// in `registry`. Inherent homonyms are left alone (`trait_impl_method_keys`).
pub(crate) fn write_back_trait_owned_string_to_impls(registry: &mut SignatureRegistry) -> usize {
    let (updates, walked) = trait_owned_string_impl_updates(registry);
    for (key, sig) in updates {
        registry.add_function(key, sig);
    }
    walked
}

/// True when [`write_back_trait_owned_string_to_impls`] would not change `registry`, and the number of impl
/// keys walked. Lets a caller skip taking a writable copy of a shared registry (a full copy per file).
pub(crate) fn trait_owned_string_write_back_is_noop(registry: &SignatureRegistry) -> (bool, usize) {
    let (updates, walked) = trait_owned_string_impl_updates(registry);
    (updates.iter().all(|(key, sig)| registry.add_function_is_noop(key, sig)), walked)
}

/// The `(key, signature)` writes of [`write_back_trait_owned_string_to_impls`] in order, and the number of
/// impl keys walked (the `trait_keys_scanned` work counter).
fn trait_owned_string_impl_updates(registry: &SignatureRegistry) -> (Vec<(String, FunctionSignature)>, usize) {
    let impl_keys = registry.all_trait_impl_method_keys();
    let walked = impl_keys.len();
    if impl_keys.is_empty() {
        return (Vec::new(), 0);
    }
    let mut updates: Vec<(String, FunctionSignature)> = Vec::new();
    // The registry is not written until every update is computed, so the trait definitions of one method
    // name are looked up once per call instead of once per impl method.
    let mut trait_sigs_by_method: HashMap<String, Vec<(String, FunctionSignature)>> = HashMap::new();
    for key in impl_keys {
        let Some(mut sig) = registry.get_signature(&key).cloned() else {
            continue;
        };
        let method = key.rsplit("::").next().unwrap_or(&key).to_string();
        let trait_sigs = trait_sigs_by_method
            .entry(method.clone())
            .or_insert_with(|| super::call_signature_resolution::trait_definition_sigs_for_method(registry, &method));
        super::call_signature_resolution::apply_trait_owned_string_contracts_from(registry, trait_sigs, &mut sig);
        updates.push((key, sig));
    }
    (updates, walked)
}
