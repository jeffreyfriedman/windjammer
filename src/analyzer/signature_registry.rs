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
    /// Codegen refresh: `true` when the emitted formal is `&String` (`@string_ref`),
    /// distinct from `&str` (Phase-2 / readonly text).
    pub string_ref_string_formal_params: Option<Vec<bool>>,
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
            string_ref_string_formal_params: None,
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

    /// True when codegen recorded an `&String` (`@string_ref`) formal for this arg.
    pub fn string_ref_string_formal_for_arg(&self, arg_index: usize) -> bool {
        let idx = self.arg_param_index(arg_index);
        self.string_ref_string_formal_params
            .as_ref()
            .and_then(|flags| flags.get(idx).copied())
            == Some(true)
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
    /// WJ `std::module` names populated by the runtime scanner (`http`, `csv` from
    /// `csv_mod.rs`, `async` from `async_runtime.rs`) — never a hand-maintained list.
    runtime_std_modules: HashSet<String>,
    /// WJ module name → rust file stem (`csv` → `csv_mod`, `http` → `http`).
    /// Only populated from scanned `windjammer-runtime/src/*.rs` files, not `std/*.wj`.
    runtime_std_rust_stems: HashMap<String, String>,
    /// Scanned `impl Type` in a runtime module file → WJ module (`Connection` → `db`).
    runtime_type_modules: HashMap<String, String>,
    /// Public types exported by a runtime module (`regex` → `Regex` from `pub use`).
    runtime_exported_types: HashMap<String, Vec<String>>,
    /// Scanned `pub struct` fields (`ServerRequest` → `[("query", "HashMap<…>"), …]`).
    runtime_type_fields: HashMap<String, Vec<(String, String)>>,
    /// Runtime `pub struct` names without `Copy` in `#[derive(...)]` (`Row`, `Connection`, …).
    /// Empty WJ std stubs must not be treated as Copy when indexing/moving at call sites.
    runtime_non_copy_types: HashSet<String>,
    /// Qualified callees marked `wj-taint: sanitizer` in scanned runtime source.
    taint_sanitizer_callees: HashSet<String>,
    /// Read-only fallback for cross-file lookups without cloning the full crate registry.
    global_fallback: Option<std::sync::Arc<SignatureRegistry>>,
}

impl Default for SignatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SignatureRegistry {
    /// Shared stdlib baseline (loaded once). Prefer this over `new()` when only
    /// reading stdlib / meta signatures — avoids cloning the full HashMap.
    pub fn stdlib() -> &'static SignatureRegistry {
        STDLIB_BASELINE.get_or_init(|| {
            // Runtime Rust APIs (`&str` / `AsRef<str>`) live in the fallback layer so
            // WJ `.wj` / stdlib_meta stubs that declare owned `string` can shadow them
            // in `signatures` without erasing the borrow contract — call sites consult
            // `get_fallback_signature` via prefer_shared_ref.
            let mut runtime = SignatureRegistry::empty();

            if let Err(e) = crate::stdlib_scanner::populate_runtime_signatures(&mut runtime) {
                eprintln!("Warning: Failed to scan runtime signatures: {}", e);
                eprintln!("Continuing with empty registry - may generate incorrect borrows");
            }

            let mut registry = SignatureRegistry::layered(std::sync::Arc::new(runtime));
            Self::load_stdlib_meta(&mut registry);
            super::primitive_float_signatures::register_primitive_float_signatures(&mut registry);
            registry
        })
    }

    pub fn new() -> Self {
        Self::stdlib().clone()
    }

    /// Lightweight empty registry (no stdlib) for building deltas.
    pub fn empty() -> Self {
        SignatureRegistry {
            signatures: HashMap::new(),
            type_collision_keys: HashSet::new(),
            ownership_collision_keys: HashSet::new(),
            method_index: HashMap::new(),
            trait_method_keys: HashSet::new(),
            runtime_std_modules: HashSet::new(),
            runtime_std_rust_stems: HashMap::new(),
            runtime_type_modules: HashMap::new(),
            runtime_exported_types: HashMap::new(),
            runtime_type_fields: HashMap::new(),
            runtime_non_copy_types: HashSet::new(),
            taint_sanitizer_callees: HashSet::new(),
            global_fallback: None,
        }
    }

    /// Local registry with read-through to a shared global registry (O(1) setup vs full clone).
    pub fn layered(global: std::sync::Arc<SignatureRegistry>) -> Self {
        let mut registry = Self::empty();
        registry.global_fallback = Some(global);
        registry
    }

    /// Record a scanned WJ `std::module` name (`csv_mod` also registers `csv`).
    /// Identity only — does not map import paths to `windjammer_runtime::{stem}`.
    pub fn register_runtime_std_module(&mut self, rust_stem: &str) {
        if rust_stem.is_empty() {
            return;
        }
        self.runtime_std_modules.insert(rust_stem.to_string());
        if let Some(short) = rust_stem.strip_suffix("_mod") {
            self.runtime_std_modules.insert(short.to_string());
        }
        if let Some(short) = rust_stem.strip_suffix("_runtime") {
            self.runtime_std_modules.insert(short.to_string());
        }
    }

    /// Record a runtime `.rs` file stem used for `use windjammer_runtime::{stem}`.
    pub fn register_runtime_file_stem(&mut self, rust_stem: &str) {
        self.register_runtime_std_module(rust_stem);
        self.insert_runtime_rust_stem(rust_stem, rust_stem);
        if let Some(short) = rust_stem.strip_suffix("_mod") {
            self.insert_runtime_rust_stem(short, rust_stem);
        }
        if let Some(short) = rust_stem.strip_suffix("_runtime") {
            self.insert_runtime_rust_stem(short, rust_stem);
        }
    }

    fn insert_runtime_rust_stem(&mut self, wj_name: &str, rust_stem: &str) {
        match self.runtime_std_rust_stems.get(wj_name) {
            Some(existing) if existing.len() >= rust_stem.len() => {}
            _ => {
                self.runtime_std_rust_stems
                    .insert(wj_name.to_string(), rust_stem.to_string());
            }
        }
    }

    /// Record `impl Type` from a runtime module file (`Connection` in `db.rs` → `db`).
    pub fn register_runtime_type_module(&mut self, type_name: &str, module: &str) {
        let wj = module
            .strip_suffix("_mod")
            .or_else(|| module.strip_suffix("_runtime"))
            .unwrap_or(module);
        if type_name.is_empty() || wj.is_empty() {
            return;
        }
        self.runtime_type_modules
            .insert(type_name.to_string(), wj.to_string());
        self.register_runtime_std_module(module);
        self.register_runtime_exported_type(module, type_name);
    }

    /// Record a public type (`pub struct` / `pub enum` / `pub use …::T`) on a runtime module.
    pub fn register_runtime_exported_type(&mut self, module: &str, type_name: &str) {
        let wj = module
            .strip_suffix("_mod")
            .or_else(|| module.strip_suffix("_runtime"))
            .unwrap_or(module);
        if type_name.is_empty() || wj.is_empty() {
            return;
        }
        if !type_name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        {
            return;
        }
        for key in [wj, module] {
            let entry = self
                .runtime_exported_types
                .entry(key.to_string())
                .or_default();
            if !entry.iter().any(|t| t == type_name) {
                entry.push(type_name.to_string());
            }
        }
    }

    /// Runtime rust file stem for a WJ `std::module` (`csv` → `csv_mod`).
    pub fn runtime_rust_stem(&self, wj_name: &str) -> Option<&str> {
        if let Some(s) = self.runtime_std_rust_stems.get(wj_name) {
            return Some(s.as_str());
        }
        self.global_fallback
            .as_ref()
            .and_then(|g| g.runtime_rust_stem(wj_name))
    }

    /// True when `name` is a scanned WJ runtime std module (not a user module).
    pub fn has_runtime_std_module(&self, name: &str) -> bool {
        if self.runtime_std_modules.contains(name) {
            return true;
        }
        self.global_fallback
            .as_ref()
            .is_some_and(|g| g.has_runtime_std_module(name))
    }

    /// Runtime module for a scanned impl type (`Connection` → `db`).
    pub fn runtime_module_for_type(&self, type_name: &str) -> Option<&str> {
        let base = type_name.rsplit("::").next().unwrap_or(type_name);
        if let Some(m) = self.runtime_type_modules.get(base) {
            return Some(m.as_str());
        }
        self.global_fallback
            .as_ref()
            .and_then(|g| g.runtime_module_for_type(type_name))
    }

    /// Resolve WJ std stub name → runtime Rust fn segment, checking local registry,
    /// optional global multipass registry, then the process-wide stdlib snapshot.
    pub fn resolve_runtime_emit_method_name_chain(
        wj_qualified: &str,
        local: &Self,
        global: Option<&Self>,
    ) -> Option<String> {
        local
            .resolve_runtime_emit_method_name(wj_qualified)
            .or_else(|| global.and_then(|g| g.resolve_runtime_emit_method_name(wj_qualified)))
            .or_else(|| Self::stdlib().resolve_runtime_emit_method_name(wj_qualified))
    }

    /// Resolve emit alias from method name alone when exactly one stdlib type maps
    /// unambiguously (e.g. `DirEntry::name` → `file_name`).
    pub fn resolve_runtime_emit_method_name_unambiguous(
        wj_method: &str,
        local: &Self,
        global: Option<&Self>,
    ) -> Option<String> {
        let suffix = format!("::{wj_method}");
        let mut emit_names = Vec::new();
        for key in Self::stdlib().signatures.keys() {
            if key.ends_with(&suffix) && key.contains("::") {
                if let Some(emit) =
                    Self::resolve_runtime_emit_method_name_chain(key, local, global)
                {
                    emit_names.push(emit);
                }
            }
        }
        emit_names.sort();
        emit_names.dedup();
        if emit_names.len() == 1 {
            emit_names.into_iter().next()
        } else {
            None
        }
    }

    /// When a WJ std stub (`random::range`) shadows a differently-named runtime fn
    /// (`random::int_range`), return the runtime Rust method segment for codegen.
    pub fn resolve_runtime_emit_method_name(&self, wj_qualified: &str) -> Option<String> {
        if !self.signatures.contains_key(wj_qualified) {
            return None;
        }
        if self
            .global_fallback
            .as_ref()
            .is_some_and(|g| g.get_signature(wj_qualified).is_some())
        {
            return None;
        }
        let parts: Vec<&str> = wj_qualified.split("::").collect();
        let (match_module, wj_method) = match parts.as_slice() {
            [module, ty, method] if self.has_runtime_std_module(module) => (*ty, *method),
            [head, method] if self.has_runtime_std_module(head) => (*head, *method),
            [ty, method] if self.runtime_module_for_type(ty).is_some() => (*ty, *method),
            _ => return None,
        };
        let wj_sig = self.get_signature(wj_qualified)?;
        let fallback = self.global_fallback.as_ref()?;
        let mut hits: Vec<String> = Vec::new();
        for (key, sig) in &fallback.signatures {
            let Some((rt_module, rt_method)) = key.rsplit_once("::") else {
                continue;
            };
            if rt_module != match_module || rt_method == wj_method {
                continue;
            }
            if wj_sig.has_self_receiver {
                if !sig.has_self_receiver
                    || wj_sig.param_types.len() != sig.param_types.len()
                    || !Self::wj_runtime_return_compatible(&wj_sig.return_type, &sig.return_type)
                {
                    continue;
                }
                let renamed_runtime_method = rt_method != wj_method
                    && rt_method.ends_with(wj_method)
                    && rt_method
                        .as_bytes()
                        .get(rt_method.len().wrapping_sub(wj_method.len()).saturating_sub(1))
                        .is_some_and(|&b| b == b'_');
                if rt_method != wj_method && !renamed_runtime_method {
                    continue;
                }
                let wj_args = wj_sig.param_types.get(1..).unwrap_or(&[]);
                let rt_args = sig.param_types.get(1..).unwrap_or(&[]);
                if Self::wj_runtime_param_lists_compatible(wj_args, rt_args) {
                    hits.push(rt_method.to_string());
                }
                continue;
            }
            if sig.has_self_receiver {
                continue;
            }
            if sig.param_types.len() == wj_sig.param_types.len()
                && rt_method.ends_with(wj_method)
                && Self::wj_runtime_param_lists_compatible(&wj_sig.param_types, &sig.param_types)
            {
                hits.push(rt_method.to_string());
            }
        }
        if hits.len() == 1 {
            Some(hits.remove(0))
        } else {
            None
        }
    }

    fn wj_runtime_return_compatible(wj: &Option<Type>, rt: &Option<Type>) -> bool {
        match (wj.as_ref(), rt.as_ref()) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                fn peel(t: &Type) -> Type {
                    match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => {
                            (**inner).clone()
                        }
                        other => other.clone(),
                    }
                }
                let a = peel(a);
                let b = peel(b);
                a == b
                    || (matches!(a, Type::String) && matches!(b, Type::String))
                    || (matches!(a, Type::Custom(n) if n.as_str() == "string" || n == "String")
                        && matches!(b, Type::Custom(m) if m.as_str() == "string" || m == "String"))
            }
            _ => false,
        }
    }

    fn wj_runtime_param_lists_compatible(wj: &[Type], rt: &[Type]) -> bool {
        fn peel(t: &Type) -> Type {
            match t {
                Type::Reference(inner) | Type::MutableReference(inner) => (**inner).clone(),
                other => other.clone(),
            }
        }
        fn is_int_like(t: &Type) -> bool {
            match peel(t) {
                Type::Int | Type::Int32 | Type::Uint => true,
                Type::Custom(name) => {
                    matches!(name.as_str(), "int" | "i64" | "i32" | "isize" | "usize")
                }
                _ => false,
            }
        }
        fn is_float_like(t: &Type) -> bool {
            match peel(t) {
                Type::Float => true,
                Type::Custom(name) => matches!(name.as_str(), "float" | "f64" | "f32"),
                _ => false,
            }
        }
        wj.len() == rt.len()
            && wj.iter().zip(rt.iter()).all(|(a, b)| {
                peel(a) == peel(b) || (is_int_like(a) && is_int_like(b)) || (is_float_like(a) && is_float_like(b))
            })
    }

    /// Fully-qualified Rust path for a scanned runtime type
    /// (`HttpMethod` → `windjammer_runtime::http::HttpMethod`).
    pub fn runtime_rust_path_for_type(&self, type_name: &str) -> Option<String> {
        let module = self.runtime_module_for_type(type_name)?;
        let stem = self.runtime_rust_stem(module).unwrap_or(module);
        let base = type_name.rsplit("::").next().unwrap_or(type_name);
        Some(format!("windjammer_runtime::{stem}::{base}"))
    }

    /// Public types exported by a scanned runtime module (`regex` → `["Regex"]`).
    pub fn runtime_exported_types_for_module(&self, wj_name: &str) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        if let Some(types) = self.runtime_exported_types.get(wj_name) {
            out.extend(types.iter().map(String::as_str));
        }
        if let Some(g) = &self.global_fallback {
            for t in g.runtime_exported_types_for_module(wj_name) {
                if !out.contains(&t) {
                    out.push(t);
                }
            }
        }
        out
    }

    /// Record scanned struct fields for a runtime type.
    pub fn register_runtime_type_fields(&mut self, type_name: &str, fields: Vec<(String, String)>) {
        if type_name.is_empty() || fields.is_empty() {
            return;
        }
        self.runtime_type_fields
            .insert(type_name.to_string(), fields);
    }

    /// Scanned fields of a runtime struct (`ServerRequest` → query/headers/body).
    pub fn runtime_type_fields(&self, type_name: &str) -> &[(String, String)] {
        let base = type_name.rsplit("::").next().unwrap_or(type_name);
        if let Some(f) = self.runtime_type_fields.get(base) {
            return f.as_slice();
        }
        self.global_fallback
            .as_ref()
            .map(|g| g.runtime_type_fields(type_name))
            .unwrap_or(&[])
    }

    /// Record a runtime struct that does not derive `Copy` (scanned from `#[derive(...)]`).
    pub fn register_runtime_non_copy_type(&mut self, type_name: &str) {
        if type_name.is_empty() {
            return;
        }
        self.runtime_non_copy_types.insert(type_name.to_string());
    }

    /// True when a scanned runtime struct is not `Copy` in Rust (`Row`, `Connection`, …).
    pub fn runtime_type_is_non_copy(&self, type_name: &str) -> bool {
        let base = type_name.rsplit("::").next().unwrap_or(type_name);
        if self.runtime_non_copy_types.contains(base) {
            return true;
        }
        self.global_fallback
            .as_ref()
            .is_some_and(|g| g.runtime_type_is_non_copy(type_name))
    }

    /// All runtime struct names registered as non-`Copy`.
    pub fn runtime_non_copy_types(&self) -> impl Iterator<Item = &str> {
        self.runtime_non_copy_types.iter().map(String::as_str)
    }

    /// Mark a scanned callee as a taint sanitizer (`/// wj-taint: sanitizer`).
    pub fn register_taint_sanitizer(&mut self, qualified_name: &str) {
        if qualified_name.is_empty() {
            return;
        }
        self.taint_sanitizer_callees
            .insert(qualified_name.to_string());
        if !qualified_name.starts_with("std::") {
            self.taint_sanitizer_callees
                .insert(format!("std::{qualified_name}"));
        }
    }

    /// True when the callee was scanned as a taint sanitizer.
    pub fn is_taint_sanitizer(&self, qualified_name: &str) -> bool {
        if self.taint_sanitizer_callees.contains(qualified_name) {
            return true;
        }
        let simple = qualified_name.rsplit("::").next().unwrap_or(qualified_name);
        let suffix = format!("::{simple}");
        if self
            .taint_sanitizer_callees
            .iter()
            .any(|k| k == qualified_name || k.ends_with(&suffix))
        {
            return true;
        }
        self.global_fallback
            .as_ref()
            .is_some_and(|g| g.is_taint_sanitizer(qualified_name))
    }

    fn load_stdlib_meta(registry: &mut Self) {
        use std::path::Path;

        let candidates = [
            Path::new("stdlib_meta").to_path_buf(),
            Path::new(env!("CARGO_MANIFEST_DIR")).join("stdlib_meta"),
        ];

        for dir in &candidates {
            if dir.is_dir() {
                // Load ONLY files under stdlib_meta/. Do not use
                // `merge_wj_meta_signatures_from_dir`, which also pulls the
                // project `.wj-cache/` (including test fixtures like
                // `MyRenderer::render`) and poisons unqualified consensus
                // (`method_mutates_receiver("render")` → true).
                crate::metadata::merge_wj_meta_signatures_from_dir_only(dir, registry);
                Self::register_wj_runtime_name_aliases(registry);
                return;
            }
        }
    }

    /// WJ std stubs whose exported name differs from the scanned runtime Rust fn.
    fn register_wj_runtime_name_aliases(registry: &mut SignatureRegistry) {
        let aliases = [
            ("random::range", "random::int_range"),
            ("DirEntry::name", "DirEntry::file_name"),
        ];
        let pending: Vec<(&str, &str)> = aliases
            .iter()
            .copied()
            .filter(|(wj_name, _)| !registry.signatures.contains_key(*wj_name))
            .collect();
        if pending.is_empty() {
            return;
        }
        let Some(fallback) = registry.global_fallback.clone() else {
            return;
        };
        let mut to_add = Vec::new();
        for (wj_name, rt_name) in pending {
            let Some(mut alias) = fallback.get_signature(rt_name).cloned() else {
                continue;
            };
            alias.name = wj_name.to_string();
            to_add.push((wj_name.to_string(), alias));
        }
        for (name, sig) in to_add {
            registry.add_function(name, sig);
        }
    }

    pub fn add_function(&mut self, name: String, sig: FunctionSignature) {
        if let Some(existing) = self.signatures.get(&name) {
            // `http::post` (module path) vs `Router::post` (Type::method): free functions
            // win under lowercase module keys. Self methods keep `Type::method` only.
            let module_path_key = name.contains("::")
                && !crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                    &name,
                );
            if module_path_key && !existing.has_self_receiver && sig.has_self_receiver {
                return;
            }
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
                if module_path_key {
                    // Free function replaces a Self method wrongly stored under module::.
                } else {
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
    /// Local signatures plus all fallback-chain entries not shadowed locally.
    /// Nested `layered` registries (file → crate → stdlib) must be fully visible
    /// so suffix consensus sees `HashMap::remove` / `Vec::push`.
    pub fn all_signatures_for_suffix_search(
        &self,
    ) -> impl Iterator<Item = (&String, &FunctionSignature)> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut current = Some(self);
        while let Some(reg) = current {
            for (k, v) in &reg.signatures {
                if seen.insert(k.as_str()) {
                    out.push((k, v));
                }
            }
            current = reg.global_fallback.as_deref();
        }
        out.into_iter()
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
    ///
    /// Prefer [`Self::find_unique_signature_ending_with`] when the result drives
    /// ownership/storage decisions — the first hit among homonyms is unsafe.
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

    /// All registered qualified keys for a bare method name (e.g. `push` → `Vec::push`, …).
    pub fn method_keys_for(&self, method: &str) -> Option<&[String]> {
        self.method_index
            .get(method)
            .map(|v| v.as_slice())
            .or_else(|| {
                self.global_fallback
                    .as_ref()
                    .and_then(|g| g.method_keys_for(method))
            })
    }

    /// `(key, sig)` for every signature whose key ends with `::{method}`, walking the
    /// global-fallback chain and deduplicating like [`Self::all_signatures_for_suffix_search`].
    ///
    /// Uses the method index at each registry layer — O(matching keys), not O(registry size).
    pub fn signatures_for_method_name(
        &self,
        method: &str,
    ) -> impl Iterator<Item = (&str, &FunctionSignature)> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(self);
        while let Some(reg) = current {
            if let Some(keys) = reg.method_index.get(method) {
                for key in keys {
                    if seen.insert(key.as_str()) {
                        if let Some(sig) = reg.signatures.get(key) {
                            out.push((key.as_str(), sig));
                        }
                    }
                }
            }
            current = reg.global_fallback.as_deref();
        }
        out.into_iter()
    }

    /// `(key, sig)` for signatures whose qualified key ends with `suffix` (e.g. `::Vec::push`),
    /// using the method index on the trailing name segment — not a full registry scan.
    pub fn signatures_matching_suffix(
        &self,
        suffix: &str,
    ) -> impl Iterator<Item = (&str, &FunctionSignature)> {
        let method = suffix
            .rsplit("::")
            .next()
            .unwrap_or(suffix)
            .trim_start_matches(':');
        let mut out = Vec::new();
        for (key, sig) in self.signatures_for_method_name(method) {
            if key.ends_with(suffix) {
                out.push((key, sig));
            }
        }
        out.into_iter()
    }

    /// `(key, sig)` for signatures whose qualified key starts with `receiver::`.
    ///
    /// Scans method-index buckets (unique method names), not the full signature map.
    pub fn signatures_for_receiver_prefix(
        &self,
        receiver_prefix: &str,
    ) -> impl Iterator<Item = (&str, &FunctionSignature)> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(self);
        while let Some(reg) = current {
            for keys in reg.method_index.values() {
                for key in keys {
                    if !key.starts_with(receiver_prefix) {
                        continue;
                    }
                    if seen.insert(key.as_str()) {
                        if let Some(sig) = reg.signatures.get(key) {
                            out.push((key.as_str(), sig));
                        }
                    }
                }
            }
            current = reg.global_fallback.as_deref();
        }
        out.into_iter()
    }

    /// Like [`Self::find_signature_ending_with`], but only when exactly one
    /// `{Type}::{suffix}` is registered (locally or via global fallback).
    pub fn find_unique_signature_ending_with(&self, suffix: &str) -> Option<&FunctionSignature> {
        let local = self
            .method_index
            .get(suffix)
            .map(|keys| {
                let mut seen = std::collections::HashSet::new();
                keys.iter()
                    .filter(|k| seen.insert(k.as_str()))
                    .filter_map(|k| self.signatures.get(k).map(|sig| (k.as_str(), sig)))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if local.len() == 1 {
            return Some(local[0].1);
        }
        if !local.is_empty() {
            return None;
        }
        self.global_fallback
            .as_ref()
            .and_then(|g| g.find_unique_signature_ending_with(suffix))
    }

    /// Find a signature matching the simple name with a specific argument count.
    /// Uses the method index for fast qualified-name lookup.
    ///
    /// When multiple distinct receivers share the same simple name + arg count
    /// (`Vec::insert` / `HashMap::insert` / `VecDeque::insert`), returns `None`
    /// so callers must resolve with a receiver type — never an arbitrary homonym.
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
            let mut matches: Vec<&FunctionSignature> = Vec::new();
            for key in keys {
                if let Some(sig) = self.signatures.get(key) {
                    let sig_args = if sig.has_self_receiver {
                        sig.param_ownership.len().saturating_sub(1)
                    } else {
                        sig.param_ownership.len()
                    };
                    if sig_args == arg_count {
                        matches.push(sig);
                    }
                }
            }
            if matches.len() == 1 {
                return Some(matches[0]);
            }
            if matches.len() > 1 {
                return None;
            }
        }
        self.global_fallback
            .as_ref()
            .and_then(|g| g.find_signature_by_name_and_arg_count(name, arg_count))
    }

    /// True when multiple `*::{method}` entries share `arg_count` but disagree on the
    /// first user-arg ownership (`Owned` vs `Borrowed`). Suffix-resolving without a
    /// receiver type would pick an arbitrary homonym (e.g. `Vec::remove` vs `HashMap::remove`).
    pub fn suffix_has_conflicting_first_arg_ownership(
        &self,
        method: &str,
        arg_count: usize,
    ) -> bool {
        let mut seen: Vec<OwnershipMode> = Vec::new();
        for (_key, sig) in self.signatures_for_method_name(method) {
            let user_args = if sig.has_self_receiver {
                sig.param_ownership.len().saturating_sub(1)
            } else {
                sig.param_ownership.len()
            };
            if user_args != arg_count {
                continue;
            }
            let first_user = if sig.has_self_receiver { 1 } else { 0 };
            if let Some(&own) = sig.param_ownership.get(first_user) {
                if !seen.iter().any(|o| *o == own) {
                    seen.push(own);
                }
            }
        }
        seen.len() > 1
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
        self.global_fallback
            .as_ref()
            .and_then(|g| g.find_delegation_callee(caller_qualified, method, arg_count))
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
        for candidate in
            crate::codegen::rust::stdlib_method_traits::stdlib_receiver_lookup_candidates(type_name)
        {
            if candidate == type_name {
                continue;
            }
            let alt = format!("{candidate}::{method}");
            if let Some(sig) = self.get_signature(&alt) {
                if Self::sig_user_arg_count(sig) == arg_count {
                    return Some(sig);
                }
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

    /// Register `crate_key::fn` aliases for every bare free-function signature.
    ///
    /// External crate metadata stores bare keys (`circuit_delta_from_edge_inserts`);
    /// call sites may use `wdb_circuit::circuit_delta_from_edge_inserts`. Aliasing
    /// keeps IR fail-closed exact-key lookup working without hardcoding API names
    /// (WDB-094).
    pub fn register_crate_prefix_aliases(&mut self, crate_key: &str) {
        if crate_key.is_empty() || crate_key.contains("::") {
            return;
        }
        let bare: Vec<(String, FunctionSignature)> = self
            .signatures
            .iter()
            .filter(|(name, _)| !name.contains("::"))
            .map(|(name, sig)| (name.clone(), sig.clone()))
            .collect();
        for (name, sig) in bare {
            let qualified = format!("{crate_key}::{name}");
            if !self.signatures.contains_key(&qualified) {
                self.add_function(qualified, sig);
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
                            string_ref_string_formal_params: None,
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
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    /// Returns true when every ownership difference between `base` and `refined`
    /// is an Owned→Borrowed or Owned→MutBorrowed refinement on a non-text type
    /// String/`string` ↔ `&str` / `Reference(string)` is a single-function codegen
    /// text-formal refinement (discard-only `path: string` → `path: &str`), not a
    /// cross-module type collision (regression-049).
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
                    // refinement (regression-049), not a cross-module type collision.
                    let text_formal_refinement =
                        Self::is_text_formal_type_refinement(existing, sig)
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
            // Never let a bare-Owned stub replace stdlib/meta `&T` key contracts
            // (`HashMap::get(key: &K)` poisoned by `std/collections.wj` `key: K`).
            if let Some(existing) = self.signatures.get(name) {
                if crate::codegen::rust::signature_promotion::existing_has_stronger_shared_ref_contract(
                    existing, sig,
                ) {
                    continue;
                }
            }
            self.signatures.insert(name.clone(), sig.clone());
        }
        self.type_collision_keys
            .extend(other.type_collision_keys.iter().cloned());
        self.ownership_collision_keys
            .extend(other.ownership_collision_keys.iter().cloned());
        self.trait_method_keys
            .extend(other.trait_method_keys.iter().cloned());
        self.runtime_std_modules
            .extend(other.runtime_std_modules.iter().cloned());
        self.runtime_std_rust_stems
            .extend(other.runtime_std_rust_stems.clone());
        self.runtime_type_modules
            .extend(other.runtime_type_modules.clone());
        for (module, types) in &other.runtime_exported_types {
            let entry = self
                .runtime_exported_types
                .entry(module.clone())
                .or_default();
            for t in types {
                if !entry.contains(t) {
                    entry.push(t.clone());
                }
            }
        }
        for (ty, fields) in &other.runtime_type_fields {
            self.runtime_type_fields
                .entry(ty.clone())
                .or_insert_with(|| fields.clone());
        }
        self.runtime_non_copy_types
            .extend(other.runtime_non_copy_types.iter().cloned());
        self.taint_sanitizer_callees
            .extend(other.taint_sanitizer_callees.iter().cloned());
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
            string_ref_string_formal_params: None,
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
            string_ref_string_formal_params: None,
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
        assert!(
            crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                sig, 1
            )
        );
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
        assert!(
            crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                sig, 0
            )
        );
    }

    #[test]
    fn test_analyzed_json_get_keeps_runtime_borrow() {
        use crate::analyzer::Analyzer;
        use crate::lexer::Lexer;
        use crate::parser::Parser;

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
        let sig = registry.get_signature("json::get").expect("json::get");
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
        use crate::analyzer::Analyzer;
        use crate::lexer::Lexer;
        use crate::parser::Parser;

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

    #[test]
    fn find_method_on_windjammer_string_receiver_resolves_string_stdlib_meta() {
        let reg = SignatureRegistry::new();
        let sig = reg
            .find_method_on_receiver_type("string", "find", 1)
            .expect("string receiver must resolve String::find from stdlib_meta");
        assert!(
            crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
                sig, 0,
            ),
            "find pattern arg must be &str in registry"
        );
    }

    #[test]
    fn resolve_runtime_emit_method_name_maps_random_range_to_int_range() {
        let reg = SignatureRegistry::stdlib();
        assert_eq!(
            reg.resolve_runtime_emit_method_name("random::range").as_deref(),
            Some("int_range"),
            "WJ random::range stub must codegen to scanned int_range"
        );
    }

    #[test]
    fn resolve_runtime_emit_method_name_maps_dir_entry_name_to_file_name() {
        let reg = SignatureRegistry::stdlib();
        assert_eq!(
            reg.resolve_runtime_emit_method_name("DirEntry::name").as_deref(),
            Some("file_name"),
            "WJ fs DirEntry::name stub must codegen to runtime file_name()"
        );
    }
}
