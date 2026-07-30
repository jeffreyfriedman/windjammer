//! Function signature storage and lookup for ownership inference.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::parser::Type;

use super::OwnershipMode;

/// True when `a` and `b` differ only by shared/mut ref wrapping of the same inner type
/// (codegen ownership demotion of one function, not a cross-module overload).
fn param_types_are_ownership_refinement(a: &Type, b: &Type) -> bool {
    if a == b {
        return true;
    }
    fn peel(t: &Type) -> Type {
        match t {
            Type::Reference(inner) | Type::MutableReference(inner) => (**inner).clone(),
            other => other.clone(),
        }
    }
    peel(a) == peel(b)
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub param_types: Vec<Type>,
    /// AST/metadata param types — never mutated by body Phase 3 Reference wrap.
    pub formal_param_types: Vec<Type>,
    pub param_ownership: Vec<OwnershipMode>,
    pub return_type: Option<Type>,
    pub return_ownership: OwnershipMode,
    pub has_self_receiver: bool,
    pub is_extern: bool,
    /// Codegen refresh: `true` when generated Rust emits `&T` for this param index.
    pub emitted_rust_ref_params: Option<Vec<bool>>,
    /// Callee params returned via field extraction only (`key.bytes`); not a full move for callers.
    pub field_extract_params: Option<Vec<bool>>,
    /// WJ-owned formal that only forwards the param to a borrowing callee (`has_key` → `get`).
    pub forwarding_borrow_params: Option<Vec<bool>>,
}

impl Default for FunctionSignature {
    fn default() -> Self {
        Self {
            name: String::new(),
            param_types: Vec::new(),
            formal_param_types: Vec::new(),
            param_ownership: Vec::new(),
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }
}

impl FunctionSignature {
    /// True when `param_types`/`param_ownership` include a leading `Self` slot for
    /// instance methods, even if `has_self_receiver` was lost in metadata export.
    pub fn has_self_receiver_slot(&self) -> bool {
        if self.has_self_receiver {
            return true;
        }
        self.param_types
            .first()
            .is_some_and(|t| matches!(t, Type::Custom(name) if name == "Self"))
    }

    fn self_receiver_slot_count(&self) -> usize {
        usize::from(self.has_self_receiver_slot())
    }

    /// Map a call-site argument index to the corresponding parameter index,
    /// accounting for implicit `self` receivers.
    ///
    /// When a self slot is present, `param_ownership[0]` and `param_types[0]`
    /// correspond to `self`, so the first user-supplied argument maps to index 1.
    pub fn arg_param_index(&self, arg_index: usize) -> usize {
        arg_index + self.self_receiver_slot_count()
    }

    /// Get the ownership mode for a call-site argument, accounting for `self`.
    pub fn param_ownership_for_arg(&self, arg_index: usize) -> Option<&OwnershipMode> {
        self.param_ownership.get(self.arg_param_index(arg_index))
    }

    /// Get the type for a call-site argument, accounting for `self`.
    pub fn param_type_for_arg(&self, arg_index: usize) -> Option<&Type> {
        self.param_types.get(self.arg_param_index(arg_index))
    }

    /// Formal parameter type at `idx` (AST/metadata). Falls back to `param_types` when
    /// `formal_param_types` is empty (legacy signatures).
    pub fn formal_param_type(&self, idx: usize) -> Option<&Type> {
        if !self.formal_param_types.is_empty() {
            self.formal_param_types.get(idx)
        } else {
            self.param_types.get(idx)
        }
    }

    /// Formal parameter type for a call-site argument index, accounting for `self`.
    pub fn formal_param_type_for_arg(&self, arg_index: usize) -> Option<&Type> {
        self.formal_param_type(self.arg_param_index(arg_index))
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
    /// Keys registered from trait definitions (e.g. `AccountReader::list_accounts`).
    /// Used by `apply_trait_owned_string_call_site_contracts` to avoid matching
    /// unrelated impl methods with the same name suffix.
    trait_method_keys: HashSet<String>,
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
                trait_method_keys: HashSet::new(),
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
            trait_method_keys: HashSet::new(),
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
            trait_method_keys: HashSet::new(),
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
                // Same-function codegen refresh: wrapping `T` ↔ `Reference(T)` /
                // `MutableReference(T)` is ownership demotion, not a namespace
                // collision. Marking `check` / `process` as type collisions here
                // strips call-site `&` for demoted formals (bug_e0308).
                let ownership_refinement = existing.param_types.len() == sig.param_types.len()
                    && existing
                        .param_types
                        .iter()
                        .zip(sig.param_types.iter())
                        .all(|(a, b)| param_types_are_ownership_refinement(a, b));
                if !stub_like && !ownership_refinement {
                    self.type_collision_keys.insert(name.clone());
                } else if ownership_refinement {
                    // Prior Owned→Reference refresh may have flagged this key; clear it.
                    self.type_collision_keys.remove(&name);
                }
            }
            // Note: ownership changes within a single registry (same file/pass)
            // are multipass refinements (Owned→Borrowed), NOT genuine collisions
            // between different modules. Only `merge()` flags cross-registry
            // ownership collisions.
            if name.contains("::") && existing.has_self_receiver && !sig.has_self_receiver {
                // Declaration stubs may incorrectly include synthetic `self`; direct impl
                // static methods must be able to replace them for Self:: call-site lowering.
                let existing_is_declaration_stub = existing
                    .param_ownership
                    .iter()
                    .all(|o| matches!(o, OwnershipMode::Owned))
                    && !existing
                        .param_types
                        .iter()
                        .any(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
                if !existing_is_declaration_stub {
                    return;
                }
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

    /// Check if a key exists in this registry's own `signatures` map (not the
    /// fallback chain).  Used by codegen to distinguish locally-registered
    /// signatures from those inherited through `global_fallback`.
    pub fn has_signature_locally(&self, name: &str) -> bool {
        self.signatures.contains_key(name)
    }

    /// Baseline signature from `global_fallback` when a local entry shadows it.
    /// Used for runtime-std auto-borrow: WJ `.wj` stubs declare owned formals while
    /// the scanned Rust runtime API takes `&T`.
    pub fn get_fallback_signature(&self, name: &str) -> Option<&FunctionSignature> {
        if !self.signatures.contains_key(name) {
            return None;
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

    /// Narrower check: only explicit ownership collision keys (from `add_function`
    /// Collect all extern function names from this registry and its global fallback.
    pub fn collect_all_extern_names(&self) -> std::collections::HashSet<String> {
        let mut names: std::collections::HashSet<String> = self
            .signatures
            .iter()
            .filter(|(_, sig)| sig.is_extern)
            .map(|(name, _)| name.rsplit("::").next().unwrap_or(name).to_string())
            .collect();
        if let Some(ref fallback) = self.global_fallback {
            names.extend(
                fallback
                    .signatures
                    .iter()
                    .filter(|(_, sig)| sig.is_extern)
                    .map(|(name, _)| name.rsplit("::").next().unwrap_or(name).to_string()),
            );
        }
        names
    }

    /// or `merge` detecting different `param_ownership`). Does NOT check
    /// `has_method_name_collision`, avoiding false positives on common method names.
    pub fn has_explicit_ownership_collision(&self, name: &str) -> bool {
        if self.ownership_collision_keys.contains(name) {
            return true;
        }
        self.global_fallback
            .as_ref()
            .is_some_and(|g| g.has_explicit_ownership_collision(name))
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
        // Same-key overwrites (`Emitter::new` from two modules) leave only one
        // surviving signature in `signatures`, but still flag type_collision_keys.
        if let Some(tn) = type_name {
            let key = format!("{tn}::{method}");
            if self.type_collision_keys.contains(&key) {
                return true;
            }
        }
        if self.has_method_name_collision_for_type(type_name, method) {
            return true;
        }
        self.global_fallback
            .as_ref()
            .is_some_and(|g| g.should_skip_int_to_float_auto_cast(type_name, method, qualified_key))
    }

    /// Like [`has_method_name_collision`] but only considers signatures whose key
    /// contains `type_name` (e.g. `Emitter` for `Emitter::new` calls).
    pub fn has_method_name_collision_for_type(
        &self,
        type_name: Option<&str>,
        method: &str,
    ) -> bool {
        // Same-key type collisions (two modules define `Emitter::new` differently)
        // overwrite `signatures` so method_index duplicate keys compare equal —
        // consult type_collision_keys first.
        if let Some(tn) = type_name {
            let key = format!("{tn}::{method}");
            if self.type_collision_keys.contains(&key) {
                return true;
            }
        }
        let Some(keys) = self.method_index.get(method) else {
            return self
                .global_fallback
                .as_ref()
                .is_some_and(|g| g.has_method_name_collision_for_type(type_name, method));
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
        self.global_fallback
            .as_ref()
            .is_some_and(|g| g.has_method_name_collision_for_type(type_name, method))
    }

    /// Returns true if the given key was registered from a trait definition.
    pub fn is_trait_method_key(&self, key: &str) -> bool {
        if self.trait_method_keys.contains(key) {
            return true;
        }
        self.global_fallback
            .as_ref()
            .is_some_and(|g| g.is_trait_method_key(key))
    }

    pub fn all_signatures(&self) -> impl Iterator<Item = (&String, &FunctionSignature)> {
        self.signatures.iter()
    }

    /// Local signatures plus global fallback entries not shadowed locally.
    /// Used by call resolution step 5c on layered registries so module-qualified
    /// keys like `quick_start::voxel_scene::VoxelScene::new` remain visible.
    pub fn all_signatures_for_suffix_search(
        &self,
    ) -> impl Iterator<Item = (&String, &FunctionSignature)> {
        let local = self.signatures.iter();
        let global = self.global_fallback.as_ref().into_iter().flat_map(|g| {
            g.signatures
                .iter()
                .filter(|(k, _)| !self.signatures.contains_key(*k))
        });
        local.chain(global)
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
        self.global_fallback
            .as_ref()
            .and_then(|g| g.find_signature_by_name_and_arg_count(name, arg_count))
    }

    fn sig_user_arg_count(sig: &FunctionSignature) -> usize {
        crate::codegen::rust::call_signature_resolution::effective_user_arg_count(sig)
    }

    /// Resolve a delegation wrapper's callee when the body is a single `receiver.method(args…)` call.
    ///
    /// Skips the caller's own qualified name (e.g. `TxnManager::get`) and returns another
    /// `{Type}::{method}` with matching user arg count (e.g. `MemoryEngine::get`).
    pub fn find_delegation_callee(
        &self,
        caller_qualified: &str,
        method: &str,
        arg_count: usize,
    ) -> Option<&FunctionSignature> {
        if let Some(keys) = self.method_index.get(method) {
            for key in keys {
                if key == caller_qualified {
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
            g.find_delegation_callee(caller_qualified, method, arg_count)
        })
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
                if !key.ends_with(&format!("::{method}")) || !key.contains(type_name) {
                    continue;
                }
                if let Some(sig) = self.signatures.get(key) {
                    if Self::sig_user_arg_count(sig) == arg_count {
                        return Some(sig);
                    }
                }
            }
        }
        self.global_fallback
            .as_ref()
            .and_then(|g| g.find_method_on_receiver_type(type_name, method, arg_count))
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
            if !module_path.is_empty() && !name.starts_with(&format!("{}::", module_path)) {
                self.add_function(format!("{}::{}", module_path, name), sig.clone());
            }
        }
    }

    /// Check if a signature's ownership has changed compared to a reference registry.
    pub fn ownership_changed(old: &FunctionSignature, new: &FunctionSignature) -> bool {
        old.param_ownership != new.param_ownership
            || old.param_types != new.param_types
            || old.return_ownership != new.return_ownership
            || old.has_self_receiver != new.has_self_receiver
    }

    /// Build declaration-only signature stubs from a parsed program (no ownership inference).
    /// Used by library multipass Step 2 to seed the global registry before Step 3 convergence.
    pub fn from_program_declarations(program: &crate::parser::Program<'_>) -> Self {
        Self::from_program_declarations_in_module(program, &[])
    }

    /// Like [`from_program_declarations`] but registers impl methods under the file's module path
    /// (`quick_start::voxel_scene::VoxelScene::new`) so homonymous types do not overwrite stubs.
    pub fn from_program_declarations_in_module(
        program: &crate::parser::Program<'_>,
        module_prefix: &[String],
    ) -> Self {
        let mut registry = Self::empty();
        Self::collect_declarations_from_items(&program.items, module_prefix, &mut registry);
        registry
    }

    fn collect_declarations_from_items(
        items: &[crate::parser::ast::core::Item<'_>],
        module_prefix: &[String],
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
                        let qualified_name = if module_prefix.is_empty() {
                            format!("{}::{}", base_type_name, func.name)
                        } else {
                            format!(
                                "{}::{}::{}",
                                module_prefix.join("::"),
                                base_type_name,
                                func.name
                            )
                        };
                        registry.add_function(qualified_name, sig.clone());
                        if module_prefix.is_empty() {
                            registry.add_function(func.name.clone(), sig);
                        }
                    }
                }
                Item::Trait { decl, .. } => {
                    for method in &decl.methods {
                        let has_self_receiver = method
                            .parameters
                            .first()
                            .is_some_and(|p| p.name == "self" || p.name == "mut self");
                        let param_types: Vec<Type> = method
                            .parameters
                            .iter()
                            .map(|p| Self::declaration_stub_param_type(p))
                            .collect();
                        let formal_param_types = param_types.clone();
                        let param_ownership: Vec<OwnershipMode> = method
                            .parameters
                            .iter()
                            .map(|p| Self::declaration_stub_param_ownership(p, false))
                            .collect();
                        let sig = FunctionSignature {
                            name: method.name.clone(),
                            param_types,
                            formal_param_types,
                            param_ownership,
                            return_type: method.return_type.clone(),
                            return_ownership: OwnershipMode::Owned,
                            has_self_receiver,
                            is_extern: false,
                            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
                        };
                        let key = format!("{}::{}", decl.name, method.name);
                        registry.trait_method_keys.insert(key.clone());
                        registry.add_function(key, sig);
                    }
                }
                Item::Mod { name, items, .. } => {
                    let mut nested = module_prefix.to_vec();
                    nested.push(name.clone());
                    Self::collect_declarations_from_items(items, &nested, registry);
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
        let param_types: Vec<Type> = func
            .parameters
            .iter()
            .map(|p| Self::declaration_stub_param_type(p))
            .collect();
        let formal_param_types = param_types.clone();
        let param_ownership: Vec<OwnershipMode> = func
            .parameters
            .iter()
            .map(|p| Self::declaration_stub_param_ownership(p, func.is_extern))
            .collect();

        FunctionSignature {
            name: name.to_string(),
            param_types,
            formal_param_types,
            param_ownership,
            return_type: func.return_type.clone(),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver,
            is_extern: func.is_extern,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    /// Returns true when every ownership difference between `base` and `refined`
    /// is an Owned→Borrowed or Owned→MutBorrowed refinement on a non-text type
    /// String/`string` ↔ `&str` / `Reference(string)` is a single-function codegen
    /// text-formal refinement (discard-only `path: string` → `path: &str`), not a
    /// cross-module type collision (WDB-049).
    fn is_text_formal_type_refinement(a: &FunctionSignature, b: &FunctionSignature) -> bool {
        if a.param_types.len() != b.param_types.len() {
            return false;
        }
        let mut saw_text_refine = false;
        for (idx, (ta, tb)) in a.param_types.iter().zip(b.param_types.iter()).enumerate() {
            if a.has_self_receiver && idx == 0 {
                continue;
            }
            if ta == tb {
                continue;
            }
            let a_owned_text = Self::is_declaration_stub_text_type(ta);
            let b_owned_text = Self::is_declaration_stub_text_type(tb);
            let a_text_ref = matches!(
                ta,
                Type::Reference(inner)
                    if Self::is_declaration_stub_text_type(inner)
                        || matches!(&**inner, Type::Custom(n) if n == "str")
            );
            let b_text_ref = matches!(
                tb,
                Type::Reference(inner)
                    if Self::is_declaration_stub_text_type(inner)
                        || matches!(&**inner, Type::Custom(n) if n == "str")
            );
            if (a_owned_text && b_text_ref) || (b_owned_text && a_text_ref) {
                saw_text_refine = true;
                continue;
            }
            return false;
        }
        saw_text_refine
    }

    /// (i.e. body analysis narrowed the ownership from the initial stub).
    ///
    /// For text types (String/string), Owned→Borrowed changes the Rust type
    /// (String → &str), so it IS a genuine collision. For non-text types like
    /// Vec<T>, Owned→Borrowed only adds `&` which auto-borrow handles.
    fn is_ownership_refinement(base: &FunctionSignature, refined: &FunctionSignature) -> bool {
        if base.param_ownership.len() != refined.param_ownership.len() {
            return false;
        }
        if base.param_types.len() != base.param_ownership.len() {
            return false;
        }
        base.param_ownership
            .iter()
            .zip(refined.param_ownership.iter())
            .zip(base.param_types.iter())
            .enumerate()
            .all(|(idx, ((b, r), ty))| {
                b == r
                    || (matches!(b, OwnershipMode::Owned)
                        && matches!(r, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                        && !Self::is_declaration_stub_text_type(ty)
                        && !(base.has_self_receiver && idx == 0))
            })
    }

    /// A signature with all params Owned and at least one non-text, non-Copy
    /// param type is almost certainly a declaration stub (pre-body-analysis),
    /// not a genuinely different function that happens to consume its arguments.
    fn is_likely_declaration_stub_with_nontrivial_params(sig: &FunctionSignature) -> bool {
        if sig.param_ownership.is_empty() {
            return false;
        }
        let all_owned = sig
            .param_ownership
            .iter()
            .all(|o| matches!(o, OwnershipMode::Owned));
        if !all_owned {
            return false;
        }
        sig.param_types.iter().enumerate().any(|(idx, ty)| {
            if sig.has_self_receiver && idx == 0 {
                return false;
            }
            !Self::is_declaration_stub_text_type(ty)
                && !matches!(ty, Type::Reference(_) | Type::MutableReference(_))
                && !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(ty)
        })
    }

    fn is_declaration_stub_text_type(ty: &Type) -> bool {
        matches!(ty, Type::String)
            || matches!(ty, Type::Custom(name) if name == "string" || name == "String")
    }

    fn declaration_stub_param_type(param: &crate::parser::Parameter<'_>) -> Type {
        // Plain `string` in an item signature is owned `String` (E0053) — do not stub as &str.
        param.type_.clone()
    }

    fn declaration_stub_param_ownership(
        param: &crate::parser::Parameter<'_>,
        is_extern: bool,
    ) -> OwnershipMode {
        match param.name.as_str() {
            "mut self" => OwnershipMode::MutBorrowed,
            "self" => OwnershipMode::Borrowed,
            _ if is_extern && Self::is_declaration_stub_text_type(&param.type_) => {
                OwnershipMode::Borrowed
            }
            _ if Self::is_declaration_stub_text_type(&param.type_) => OwnershipMode::Owned,
            _ => OwnershipMode::Owned,
        }
    }

    /// BUG #8 FIX: Merge signatures from another registry.
    /// Detects collisions when different registries provide different
    /// param types for the same key (namespace collision from different modules).
    pub fn merge(&mut self, other: &SignatureRegistry) {
        for (name, sig) in &other.signatures {
            if let Some(existing) = self.signatures.get(name) {
                // Caller-file declaration stubs must not clobber defining-module
                // codegen refresh (`emitted_rust_ref_params`, `&Vec` formals, etc.).
                if existing.emitted_rust_ref_params.is_some()
                    && sig.emitted_rust_ref_params.is_none()
                {
                    continue;
                }
                let codegen_refreshed = sig.emitted_rust_ref_params.is_some();
                if existing.param_types != sig.param_types {
                    let stub_like = existing.param_types.is_empty() || sig.param_types.is_empty();
                    // String ↔ `&str` / `Reference(string)` is a codegen text-formal
                    // refinement (WDB-049), not a cross-module type collision.
                    let text_formal_refinement = Self::is_text_formal_type_refinement(existing, sig)
                        || Self::is_text_formal_type_refinement(sig, existing);
                    if !stub_like && !text_formal_refinement {
                        self.type_collision_keys.insert(name.clone());
                    }
                } else if existing.param_ownership != sig.param_ownership {
                    // Multipass library builds register declaration stubs (all-Owned)
                    // before body analysis converges to refined ownership. When the
                    // stub is later merged with the converged signature, the ownership
                    // difference is a refinement, NOT a genuine cross-module collision.
                    //
                    // Owned→Borrowed and Owned→MutBorrowed are always valid refinements:
                    // body analysis discovered the parameter's actual usage. Only flag
                    // as collision when there is a genuine Borrowed↔MutBorrowed conflict
                    // (one module reads, another writes the same param).
                    if !Self::is_ownership_refinement(existing, sig)
                        && !Self::is_ownership_refinement(sig, existing)
                    {
                        let ex_stub =
                            Self::is_likely_declaration_stub_with_nontrivial_params(existing);
                        let sig_stub = Self::is_likely_declaration_stub_with_nontrivial_params(sig);
                        if !ex_stub && !sig_stub {
                            self.ownership_collision_keys.insert(name.clone());
                        }
                    }
                }
                if !codegen_refreshed {
                // Keep converged dependency / multi-pass ownership over per-file stubs.
                if crate::codegen::rust::signature_promotion::signature_is_declaration_stub_like(sig)
                    && !crate::codegen::rust::signature_promotion::signature_is_declaration_stub_like(
                        existing,
                    )
                    && crate::codegen::rust::signature_promotion::prefer_converged_over_stub(
                        sig, existing,
                    )
                {
                    continue;
                }
                // Cross-file: caller analysis re-registers `Type::method` with declaration
                // `Owned` metadata after the defining module already converged borrows
                // (e.g. squad.wj `Squad::new` → &str, then caller.wj merge overwrites).
                //
                // Guard: if sig introduces borrows where existing has Owned, sig is a
                // body-analysis refinement — always insert it (don't skip).
                let sig_refines_with_borrows =
                    sig.param_ownership.iter().enumerate().any(|(idx, o)| {
                        if sig.has_self_receiver && idx == 0 {
                            return false;
                        }
                        matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                            && existing
                                .param_ownership
                                .get(idx)
                                .is_some_and(|eo| matches!(eo, OwnershipMode::Owned))
                    });
                if !sig_refines_with_borrows
                    && (crate::codegen::rust::signature_promotion::prefer_converged_over_stub(
                        sig, existing,
                    ) || crate::codegen::rust::signature_promotion::global_has_borrowed_text_over_local_owned_stub(
                        sig, existing,
                    ))
                    && !crate::codegen::rust::signature_promotion::body_borrow_must_not_replace_owned_formal_stub(
                        existing, sig,
                    )
                {
                    continue;
                }
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
        self.trait_method_keys
            .extend(other.trait_method_keys.iter().cloned());
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

#[cfg(test)]
mod tests {
    use super::{FunctionSignature, OwnershipMode, SignatureRegistry};
    use crate::parser::Type;

    #[test]
    fn merge_keeps_converged_static_text_borrow_over_caller_owned_stub() {
        let converged = FunctionSignature {
            name: "Squad::new".into(),
            param_types: vec![Type::String, Type::String],
            formal_param_types: vec![Type::String, Type::String],
            param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            return_type: Some(Type::Custom("Squad".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };
        let caller_stub = FunctionSignature {
            name: "Squad::new".into(),
            param_types: vec![Type::String, Type::String],
            formal_param_types: vec![Type::String, Type::String],
            param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Owned],
            return_type: Some(Type::Custom("Squad".into())),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        };

        let mut global = SignatureRegistry::new();
        global
            .signatures
            .insert("Squad::new".into(), converged.clone());

        global.merge(&{
            let mut caller = SignatureRegistry::new();
            caller.signatures.insert("Squad::new".into(), caller_stub);
            caller
        });

        let stored = global.get_signature("Squad::new").expect("Squad::new");
        assert_eq!(
            stored.param_ownership,
            vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
            "caller owned stub must not overwrite body-converged borrows"
        );
    }

    #[test]
    fn test_runtime_subprocess_spawn_scanned_as_borrowed() {
        let reg = SignatureRegistry::new();
        let sig = reg
            .get_signature("subprocess::spawn")
            .expect("subprocess::spawn must be in stdlib baseline");
        assert!(
            sig.param_ownership.len() >= 2,
            "expected program + args params, got {:?}",
            sig.param_ownership
        );
        assert_eq!(sig.param_ownership[1], OwnershipMode::Borrowed);
        assert!(crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
            sig, 1
        ));
    }

    #[test]
    fn test_runtime_json_get_scanned_as_borrowed() {
        let reg = SignatureRegistry::new();
        let sig = reg
            .get_signature("json::get")
            .expect("json::get must be in stdlib baseline");
        assert!(
            sig.param_ownership.len() >= 1,
            "expected value param, got {:?}",
            sig.param_ownership
        );
        assert_eq!(sig.param_ownership[0], OwnershipMode::Borrowed);
        assert!(crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
            sig, 0
        ));
    }

    #[test]
    fn test_analyzed_json_get_keeps_runtime_borrow() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::analyzer::Analyzer;

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
        let program = parser.parse().unwrap();
        let mut analyzer = Analyzer::new();
        let (_, registry, _) = analyzer.analyze_program(&program).unwrap();
        let sig = registry
            .get_signature("json::get")
            .expect("json::get");
        assert!(
            crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                &registry,
                "json::get",
                Some(sig),
                0,
            ),
            "value param should need runtime auto-borrow, ownership={:?}",
            sig.param_ownership
        );
        assert!(
            crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                &registry,
                "json::get",
                None,
                0,
            ),
            "registry-only lookup must still detect runtime borrow for json::get value",
        );
    }

    #[test]
    fn test_analyzed_subprocess_use_keeps_runtime_borrow_for_spawn() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use crate::analyzer::Analyzer;

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
        let program = parser.parse().unwrap();
        let mut analyzer = Analyzer::new();
        let (_, registry, _) = analyzer.analyze_program(&program).unwrap();
        let sig = registry
            .get_signature("subprocess::spawn")
            .expect("subprocess::spawn");
        assert!(
            crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                &registry,
                "subprocess::spawn",
                Some(sig),
                1,
            ),
            "args param should need runtime auto-borrow, ownership={:?}",
            sig.param_ownership
        );
        assert!(
            crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                &registry,
                "subprocess::spawn",
                None,
                1,
            ),
            "registry-only lookup must still detect runtime borrow for spawn args",
        );
    }
}
