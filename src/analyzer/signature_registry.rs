//! Function signature storage and lookup for ownership inference.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

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

static STDLIB_BASELINE: OnceLock<SignatureRegistry> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct SignatureRegistry {
    pub signatures: HashMap<String, FunctionSignature>,
    /// Param-type mismatches (namespace collisions) — used for int→float cast safety.
    type_collision_keys: HashSet<String>,
    /// Same param types but different ownership — used for auto-borrow safety.
    ownership_collision_keys: HashSet<String>,
    method_index: HashMap<String, Vec<String>>,
    /// Read-only fallback for cross-file lookups without cloning the full crate registry.
    global_fallback: Option<std::sync::Arc<SignatureRegistry>>,
}

impl Default for SignatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SignatureRegistry {
    pub fn new() -> Self {
        let baseline = STDLIB_BASELINE.get_or_init(|| {
            let mut registry = SignatureRegistry {
                signatures: HashMap::new(),
                type_collision_keys: HashSet::new(),
                ownership_collision_keys: HashSet::new(),
                method_index: HashMap::new(),
                global_fallback: None,
            };

            if let Err(e) = crate::stdlib_scanner::populate_runtime_signatures(&mut registry) {
                eprintln!("Warning: Failed to scan runtime signatures: {}", e);
                eprintln!("Continuing with empty registry - may generate incorrect borrows");
            }

            Self::load_stdlib_meta(&mut registry);
            registry
        });

        baseline.clone()
    }

    /// Lightweight empty registry (no stdlib) for building deltas.
    pub fn empty() -> Self {
        SignatureRegistry {
            signatures: HashMap::new(),
            type_collision_keys: HashSet::new(),
            ownership_collision_keys: HashSet::new(),
            method_index: HashMap::new(),
            global_fallback: None,
        }
    }

    /// Local registry with read-through to a shared global registry (O(1) setup vs full clone).
    pub fn layered(global: std::sync::Arc<SignatureRegistry>) -> Self {
        SignatureRegistry {
            signatures: HashMap::new(),
            type_collision_keys: HashSet::new(),
            ownership_collision_keys: HashSet::new(),
            method_index: HashMap::new(),
            global_fallback: Some(global),
        }
    }

    fn load_stdlib_meta(registry: &mut Self) {
        use std::path::Path;

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
            if existing.param_types != sig.param_types {
                // Empty-param runtime/stdlib stubs (e.g. `Config::new()`) are
                // intentionally shadowed by user-defined constructors — not
                // ambiguous collisions.
                let stub_like = existing.param_types.is_empty() || sig.param_types.is_empty();
                if !stub_like {
                    self.type_collision_keys.insert(name.clone());
                }
            } else if existing.param_ownership != sig.param_ownership {
                self.ownership_collision_keys.insert(name.clone());
            }
            if name.contains("::") && existing.has_self_receiver && !sig.has_self_receiver {
                return;
            }
        }
        if let Some(suffix) = name.rsplit_once("::").map(|(_, s)| s.to_string()) {
            self.method_index
                .entry(suffix)
                .or_default()
                .push(name.clone());
        }
        self.signatures.insert(name, sig);
    }

    pub fn get_signature(&self, name: &str) -> Option<&FunctionSignature> {
        if let Some(sig) = self.signatures.get(name) {
            return Some(sig);
        }
        self.global_fallback
            .as_ref()
            .and_then(|g| g.get_signature(name))
    }

    /// Check if a signature key has been registered with conflicting param types
    /// from different modules (namespace collision).
    pub fn has_collision(&self, name: &str) -> bool {
        if self.type_collision_keys.contains(name) || self.ownership_collision_keys.contains(name) {
            return true;
        }
        if self.has_method_name_collision(name) {
            return true;
        }
        self.global_fallback
            .as_ref()
            .is_some_and(|g| g.has_collision(name))
    }

    /// True when multiple qualified methods share this suffix (e.g. `new`) with
    /// incompatible param types — used for unqualified calls like `Emitter::new`.
    pub fn has_method_name_collision(&self, method: &str) -> bool {
        self.has_method_name_collision_for_type(None, method)
    }

    /// Whether int→float auto-cast should be skipped for safety.
    ///
    /// Skips when the exact qualified key has a param-type collision, or when
    /// multiple implementations of `method` on the same (or unknown) type disagree
    /// on parameter types.
    pub fn should_skip_int_to_float_auto_cast(
        &self,
        type_name: Option<&str>,
        method: &str,
        qualified_key: Option<&str>,
    ) -> bool {
        if qualified_key.is_some_and(|k| self.type_collision_keys.contains(k)) {
            return true;
        }
        if self.has_method_name_collision_for_type(type_name, method) {
            return true;
        }
        self.global_fallback.as_ref().is_some_and(|g| {
            g.should_skip_int_to_float_auto_cast(type_name, method, qualified_key)
        })
    }

    /// Like [`has_method_name_collision`] but only considers signatures whose key
    /// contains `type_name` (e.g. `Emitter` for `Emitter::new` calls).
    pub fn has_method_name_collision_for_type(
        &self,
        type_name: Option<&str>,
        method: &str,
    ) -> bool {
        let Some(keys) = self.method_index.get(method) else {
            return self.global_fallback.as_ref().is_some_and(|g| {
                g.has_method_name_collision_for_type(type_name, method)
            });
        };
        let filtered: Vec<&String> = if let Some(tn) = type_name {
            keys.iter()
                .filter(|k| {
                    k.ends_with(&format!("::{method}"))
                        && (k.as_str() == format!("{tn}::{method}")
                            || k.contains(&format!("::{tn}::")))
                })
                .collect()
        } else {
            keys.iter().collect()
        };
        if filtered.len() >= 2 {
            let mut first: Option<&FunctionSignature> = None;
            for key in filtered {
                if let Some(sig) = self.signatures.get(key) {
                    if let Some(f) = first {
                        if f.param_types != sig.param_types
                            || f.param_ownership != sig.param_ownership
                        {
                            return true;
                        }
                    } else {
                        first = Some(sig);
                    }
                }
            }
        }
        self.global_fallback.as_ref().is_some_and(|g| {
            g.has_method_name_collision_for_type(type_name, method)
        })
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
    /// Uses the method index for O(1) lookup instead of scanning all entries.
    pub fn find_signature_ending_with(&self, suffix: &str) -> Option<&FunctionSignature> {
        if let Some(keys) = self.method_index.get(suffix) {
            for key in keys {
                if let Some(sig) = self.signatures.get(key) {
                    return Some(sig);
                }
            }
        }
        self.global_fallback
            .as_ref()
            .and_then(|g| g.find_signature_ending_with(suffix))
    }

    /// Find a signature matching the simple name with a specific argument count.
    /// Uses the method index for fast qualified-name lookup.
    pub fn find_signature_by_name_and_arg_count(
        &self,
        name: &str,
        arg_count: usize,
    ) -> Option<&FunctionSignature> {
        if let Some(sig) = self.get_signature(name) {
            let sig_args = if sig.has_self_receiver {
                sig.param_ownership.len().saturating_sub(1)
            } else {
                sig.param_ownership.len()
            };
            if sig_args == arg_count {
                return Some(sig);
            }
        }
        if let Some(keys) = self.method_index.get(name) {
            for key in keys {
                if let Some(sig) = self.signatures.get(key) {
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
        }
        self.global_fallback.as_ref().and_then(|g| {
            g.find_signature_by_name_and_arg_count(name, arg_count)
        })
    }

    fn sig_user_arg_count(sig: &FunctionSignature) -> usize {
        if sig.has_self_receiver {
            sig.param_ownership.len().saturating_sub(1)
        } else {
            sig.param_ownership.len()
        }
    }

    /// Resolve `TypeName::method` for call-site borrow coercion when homonyms exist.
    pub fn find_method_on_receiver_type(
        &self,
        type_name: &str,
        method: &str,
        arg_count: usize,
    ) -> Option<&FunctionSignature> {
        let qualified = format!("{type_name}::{method}");
        if let Some(sig) = self.get_signature(&qualified) {
            if Self::sig_user_arg_count(sig) == arg_count {
                return Some(sig);
            }
        }
        if let Some(keys) = self.method_index.get(method) {
            for key in keys {
                if !key.ends_with(&format!("::{method}"))
                    || !key.contains(type_name)
                {
                    continue;
                }
                if let Some(sig) = self.signatures.get(key) {
                    if Self::sig_user_arg_count(sig) == arg_count {
                        return Some(sig);
                    }
                }
            }
        }
        self.global_fallback.as_ref().and_then(|g| {
            g.find_method_on_receiver_type(type_name, method, arg_count)
        })
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

    /// Build declaration-only signature stubs from a parsed program (no ownership inference).
    /// Used by library multipass Step 2 to seed the global registry before Step 3 convergence.
    pub fn from_program_declarations(program: &crate::parser::Program<'_>) -> Self {
        let mut registry = Self::empty();
        Self::collect_declarations_from_items(&program.items, &mut registry);
        registry
    }

    fn collect_declarations_from_items(
        items: &[crate::parser::ast::core::Item<'_>],
        registry: &mut Self,
    ) {
        use crate::parser::ast::core::Item;

        for item in items {
            match item {
                Item::Function { decl, .. } => {
                    let sig = Self::signature_stub_from_decl(decl, &decl.name);
                    registry.add_function(decl.name.clone(), sig);
                }
                Item::Impl { block, .. } => {
                    let base_type_name = block
                        .type_name
                        .split('<')
                        .next()
                        .unwrap_or(&block.type_name);
                    for func in &block.functions {
                        let sig = Self::signature_stub_from_decl(func, &func.name);
                        let qualified_name = format!("{}::{}", base_type_name, func.name);
                        registry.add_function(qualified_name, sig.clone());
                        registry.add_function(func.name.clone(), sig);
                    }
                }
                Item::Trait { decl, .. } => {
                    for method in &decl.methods {
                        let has_self_receiver = method
                            .parameters
                            .first()
                            .is_some_and(|p| p.name == "self" || p.name == "mut self");
                        let param_types: Vec<Type> =
                            method.parameters.iter().map(|p| p.type_.clone()).collect();
                        let param_ownership = vec![OwnershipMode::Owned; param_types.len()];
                        let sig = FunctionSignature {
                            name: method.name.clone(),
                            param_types,
                            param_ownership,
                            return_type: method.return_type.clone(),
                            return_ownership: OwnershipMode::Owned,
                            has_self_receiver,
                            is_extern: false,
                        };
                        registry.add_function(format!("{}::{}", decl.name, method.name), sig);
                    }
                }
                Item::Mod { items, .. } => {
                    Self::collect_declarations_from_items(items, registry);
                }
                _ => {}
            }
        }
    }

    fn signature_stub_from_decl(
        func: &crate::parser::ast::core::FunctionDecl<'_>,
        name: &str,
    ) -> FunctionSignature {
        let has_self_receiver = func
            .parameters
            .first()
            .is_some_and(|p| p.name == "self" || p.name == "mut self");
        let param_types: Vec<Type> = func.parameters.iter().map(|p| p.type_.clone()).collect();
        let param_ownership = vec![OwnershipMode::Owned; param_types.len()];

        FunctionSignature {
            name: name.to_string(),
            param_types,
            param_ownership,
            return_type: func.return_type.clone(),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver,
            is_extern: func.is_extern,
        }
    }

    /// BUG #8 FIX: Merge signatures from another registry.
    /// Detects collisions when different registries provide different
    /// param types for the same key (namespace collision from different modules).
    pub fn merge(&mut self, other: &SignatureRegistry) {
        for (name, sig) in &other.signatures {
            if let Some(existing) = self.signatures.get(name) {
                if existing.param_types != sig.param_types {
                    let stub_like = existing.param_types.is_empty() || sig.param_types.is_empty();
                    if !stub_like {
                        self.type_collision_keys.insert(name.clone());
                    }
                } else if existing.param_ownership != sig.param_ownership {
                    self.ownership_collision_keys.insert(name.clone());
                }
            }
            if let Some(suffix) = name.rsplit_once("::").map(|(_, s)| s.to_string()) {
                self.method_index
                    .entry(suffix)
                    .or_default()
                    .push(name.clone());
            }
            self.signatures.insert(name.clone(), sig.clone());
        }
        self.type_collision_keys
            .extend(other.type_collision_keys.iter().cloned());
        self.ownership_collision_keys
            .extend(other.ownership_collision_keys.iter().cloned());
    }

    /// Collect only signatures whose ownership differs from `base`.
    /// Used by multipass Step 3 to avoid deep-cloning the full registry each round.
    pub fn delta_from_base(
        base: &SignatureRegistry,
        updated: &SignatureRegistry,
    ) -> SignatureDelta {
        let mut changed = HashMap::new();
        for (name, sig) in &updated.signatures {
            let is_new_or_changed = match base.signatures.get(name) {
                None => true,
                Some(old) => Self::ownership_changed(old, sig),
            };
            if is_new_or_changed {
                changed.insert(name.clone(), sig.clone());
            }
        }
        SignatureDelta { changed }
    }

    /// Merge a delta into this registry (changed keys only).
    pub fn merge_delta(&mut self, delta: &SignatureDelta) {
        for (name, sig) in &delta.changed {
            if let Some(suffix) = name.rsplit_once("::").map(|(_, s)| s.to_string()) {
                self.method_index
                    .entry(suffix)
                    .or_default()
                    .push(name.clone());
            }
            self.signatures.insert(name.clone(), sig.clone());
        }
    }
}

/// Ownership-only changes from one analysis pass (avoids full registry clone).
#[derive(Debug, Clone, Default)]
pub struct SignatureDelta {
    pub changed: HashMap<String, FunctionSignature>,
}
