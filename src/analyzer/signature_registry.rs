//! Function signature storage and lookup for ownership inference.

use std::collections::{HashMap, HashSet};

use crate::parser::Type;

use super::OwnershipMode;

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub param_types: Vec<Type>, // ADDED: Store actual parameter types for smart inference
    pub param_ownership: Vec<OwnershipMode>,
    pub return_type: Option<Type>, // ADDED: Store return type for smart inference
    pub return_ownership: OwnershipMode,
    pub has_self_receiver: bool, // True if first parameter is self/&self/&mut self
    pub is_extern: bool,         // True if this is an extern function (FFI)
}

#[derive(Debug, Clone)]
pub struct SignatureRegistry {
    pub signatures: HashMap<String, FunctionSignature>,
    collision_keys: HashSet<String>,
}

impl Default for SignatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SignatureRegistry {
    pub fn new() -> Self {
        let mut registry = SignatureRegistry {
            signatures: HashMap::new(),
            collision_keys: HashSet::new(),
        };

        // Populate with stdlib signatures by scanning windjammer-runtime source
        if let Err(e) = crate::stdlib_scanner::populate_runtime_signatures(&mut registry) {
            eprintln!("Warning: Failed to scan runtime signatures: {}", e);
            eprintln!("Continuing with empty registry - may generate incorrect borrows");
        }

        registry
    }

    pub fn add_function(&mut self, name: String, sig: FunctionSignature) {
        if let Some(existing) = self.signatures.get(&name) {
            if existing.param_types != sig.param_types
                || existing.param_ownership != sig.param_ownership
            {
                self.collision_keys.insert(name.clone());
            }
            // For qualified names (Type::method), protect a method signature
            // (has_self_receiver=true) from being overwritten by a standalone function
            // (has_self_receiver=false). This prevents metadata/analysis ordering races
            // from losing the correct &mut self inference.
            if name.contains("::")
                && existing.has_self_receiver
                && !sig.has_self_receiver
            {
                return;
            }
        }
        self.signatures.insert(name, sig);
    }

    pub fn get_signature(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures.get(name)
    }

    /// Check if a signature key has been registered with conflicting param types
    /// from different modules (namespace collision).
    pub fn has_collision(&self, name: &str) -> bool {
        self.collision_keys.contains(name)
    }

    pub fn all_signatures(&self) -> impl Iterator<Item = (&String, &FunctionSignature)> {
        self.signatures.iter()
    }

    /// Fallback lookup: find a signature whose key ends with `::name`.
    /// Used when a method call is recorded as bare "method" but registered as "Type::method".
    pub fn find_signature_ending_with(&self, suffix: &str) -> Option<&FunctionSignature> {
        let pattern = format!("::{}", suffix);
        self.signatures
            .iter()
            .find(|(key, _)| key.ends_with(&pattern))
            .map(|(_, sig)| sig)
    }

    /// Find a signature matching the simple name with a specific argument count.
    /// Searches exact match first, then all qualified names ending with `::name`.
    /// Used when simple-name lookup returns the wrong overload (name collision).
    pub fn find_signature_by_name_and_arg_count(
        &self,
        name: &str,
        arg_count: usize,
    ) -> Option<&FunctionSignature> {
        // Try exact match first
        if let Some(sig) = self.signatures.get(name) {
            let sig_args = if sig.has_self_receiver {
                sig.param_ownership.len().saturating_sub(1)
            } else {
                sig.param_ownership.len()
            };
            if sig_args == arg_count {
                return Some(sig);
            }
        }
        // Search all signatures ending with ::name
        let pattern = format!("::{}", name);
        for (key, sig) in &self.signatures {
            if key.ends_with(&pattern) || key == name {
                let sig_args = if sig.has_self_receiver {
                    sig.param_ownership.len().saturating_sub(1)
                } else {
                    sig.param_ownership.len()
                };
                if sig_args == arg_count {
                    return Some(sig);
                }
            }
        }
        None
    }

    /// BUG #8 FIX: Merge signatures from another registry.
    /// Detects collisions when different registries provide different
    /// param types for the same key (namespace collision from different modules).
    pub fn merge(&mut self, other: &SignatureRegistry) {
        for (name, sig) in &other.signatures {
            if let Some(existing) = self.signatures.get(name) {
                if existing.param_types != sig.param_types
                    || existing.param_ownership != sig.param_ownership
                {
                    self.collision_keys.insert(name.clone());
                }
            }
            self.signatures.insert(name.clone(), sig.clone());
        }
        self.collision_keys
            .extend(other.collision_keys.iter().cloned());
    }
}
