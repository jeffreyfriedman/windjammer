//! Function signature storage and lookup for ownership inference.

use std::collections::{HashMap, HashSet};

use crate::parser::Type;

use super::OwnershipMode;

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub param_types: Vec<Type>,
    pub param_ownership: Vec<OwnershipMode>,
    pub return_type: Option<Type>,
    pub return_ownership: OwnershipMode,
    pub has_self_receiver: bool,
    pub is_extern: bool,
}

impl FunctionSignature {
    /// Map a call-site argument index to the corresponding parameter index,
    /// accounting for implicit `self` receivers.
    ///
    /// When `has_self_receiver` is true, `param_ownership[0]` and
    /// `param_types[0]` correspond to `self`, so the first user-supplied
    /// argument maps to index 1.
    pub fn arg_param_index(&self, arg_index: usize) -> usize {
        if self.has_self_receiver {
            arg_index + 1
        } else {
            arg_index
        }
    }

    /// Get the ownership mode for a call-site argument, accounting for `self`.
    pub fn param_ownership_for_arg(&self, arg_index: usize) -> Option<&OwnershipMode> {
        self.param_ownership.get(self.arg_param_index(arg_index))
    }

    /// Get the type for a call-site argument, accounting for `self`.
    pub fn param_type_for_arg(&self, arg_index: usize) -> Option<&Type> {
        self.param_types.get(self.arg_param_index(arg_index))
    }
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

        // Load Rust stdlib type signatures from shipped .wj.meta files.
        // These provide type-qualified ownership info (e.g. Vec::push → &mut self, T)
        // so the compiler doesn't need hard-coded method name tables.
        Self::load_stdlib_meta(&mut registry);

        registry
    }

    fn load_stdlib_meta(registry: &mut Self) {
        use std::path::Path;

        // Locate stdlib_meta/ relative to the compiler binary or crate root
        let candidates = [
            Path::new("stdlib_meta").to_path_buf(),
            Path::new(env!("CARGO_MANIFEST_DIR")).join("stdlib_meta"),
        ];

        for dir in &candidates {
            if dir.is_dir() {
                crate::metadata::merge_wj_meta_signatures_from_dir(dir, registry);
                return;
            }
        }
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

    /// Look up a method by name, trying exact match first then falling back to
    /// a qualified name ending with `::name`.
    ///
    /// This is the canonical lookup most call sites should use instead of
    /// `get_signature(m).or_else(|| find_signature_ending_with(m))`.
    pub fn lookup_method(&self, name: &str) -> Option<&FunctionSignature> {
        self.get_signature(name)
            .or_else(|| self.find_signature_ending_with(name))
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

    /// Register module-qualified aliases for all signatures in `source`.
    /// For each unqualified name, registers `file_stem::name` and optionally
    /// `module_path::name` to support cross-file lookups.
    pub fn register_module_aliases(
        &mut self,
        source: &SignatureRegistry,
        file_stem: &str,
        module_path: &str,
    ) {
        if file_stem.is_empty() {
            return;
        }
        for (name, sig) in &source.signatures {
            if !name.contains("::") {
                self.add_function(format!("{}::{}", file_stem, name), sig.clone());
            }
            if !module_path.is_empty() {
                self.add_function(format!("{}::{}", module_path, name), sig.clone());
            }
        }
    }

    /// Check if a signature's ownership has changed compared to a reference registry.
    pub fn ownership_changed(old: &FunctionSignature, new: &FunctionSignature) -> bool {
        old.param_ownership != new.param_ownership
            || old.return_ownership != new.return_ownership
            || old.has_self_receiver != new.has_self_receiver
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
