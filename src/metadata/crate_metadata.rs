//! Crate-level metadata (`metadata.json`) and project cache path resolution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::analyzer::{FunctionSignature as AnalyzerFunctionSignature, OwnershipMode};
use crate::parser::Type;

use super::function_metadata::{try_analyzer_signature_from_metadata, FunctionSignature};
use super::ModuleMetadata;

fn parent_has_manifest(dir: &Path) -> bool {
    dir.parent()
        .is_some_and(|p| p.join("Cargo.toml").exists() || p.join("wj.toml").exists())
}

/// True for manifests that must not become the `.wj-cache` project root.
fn is_auxiliary_nested_manifest(dir: &Path) -> bool {
    // Auto-generated `src/Cargo.toml` inside a real crate (e.g. game_core/src/).
    if dir.file_name().and_then(|n| n.to_str()) == Some("src") && parent_has_manifest(dir) {
        return true;
    }
    // Auxiliary mini-manifests under `<crate>/src/<module>/` (e.g. src/rendering/Cargo.toml).
    if let Some(parent) = dir.parent() {
        if parent.file_name().and_then(|n| n.to_str()) == Some("src") {
            if let Some(grandparent) = parent.parent() {
                if grandparent.join("Cargo.toml").exists() || grandparent.join("wj.toml").exists() {
                    return true;
                }
            }
        }
    }
    // Compiler-emitted output manifest at `<crate>/gen/Cargo.toml` (not a real crate root).
    if dir.file_name().and_then(|n| n.to_str()) == Some("gen") && parent_has_manifest(dir) {
        return true;
    }
    false
}

fn meta_cache_path_under_root(project_root: &Path, relative: &Path) -> PathBuf {
    let mut cache_path = project_root.join(".wj-cache");
    cache_path.push(relative);
    let file_name = cache_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    cache_path.set_file_name(format!("{}.meta", file_name));
    cache_path
}

/// Find the nearest crate root by walking up from `start` looking for `Cargo.toml` or `wj.toml`.
///
/// Returns the **closest** valid manifest (not the workspace parent) and skips auxiliary
/// nested manifests so `.wj-cache` lives at the real crate root.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?
    } else {
        start
    };
    loop {
        if (dir.join("Cargo.toml").exists() || dir.join("wj.toml").exists())
            && !is_auxiliary_nested_manifest(dir)
        {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

/// Compute the cache path for a `.wj.meta` file.
///
/// Given `<project>/src/foo/bar.wj`, returns `<project>/.wj-cache/foo/bar.wj.meta`.
/// Finds the project root by walking up to the nearest `Cargo.toml` or `wj.toml`,
/// then strips the `src/` prefix to compute the relative cache path.
pub fn meta_cache_path(source_file: &Path) -> PathBuf {
    if let Some(project_root) = find_project_root(source_file) {
        let src_dir = project_root.join("src");
        if let Ok(relative) = source_file.strip_prefix(&src_dir) {
            return meta_cache_path_under_root(&project_root, relative);
        }
        // Output-dir mirrors (`gen/foo.wj`) share cache keys with `src/foo.wj`.
        let gen_dir = project_root.join("gen");
        if let Ok(relative) = source_file.strip_prefix(&gen_dir) {
            return meta_cache_path_under_root(&project_root, relative);
        }
        // File is in the project but not under src/ or gen/ -- place relative to project root
        if let Ok(relative) = source_file.strip_prefix(&project_root) {
            return meta_cache_path_under_root(&project_root, relative);
        }
    }
    // No manifest above the file: the analysis cache still never goes next to a source.
    projectless_meta_path(source_file)
}

/// Directory holding analysis metadata for sources that have no `wj.toml`/`Cargo.toml` above them.
/// `WJ_META_CACHE_DIR` overrides the default `<windjammer cache root>/meta`.
pub fn global_meta_cache_dir() -> PathBuf {
    match std::env::var_os("WJ_META_CACHE_DIR") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => crate::cargo_cache::cache_root().join("meta"),
    }
}

/// The module root of a manifest-less tree: the nearest ancestor named `src`, else the directory itself.
fn projectless_root(start: &Path) -> PathBuf {
    let dir = if start.is_file() { start.parent().unwrap_or(start) } else { start };
    let abs = dir.canonicalize().unwrap_or_else(|_| {
        if dir.is_absolute() { dir.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(dir) }
    });
    for a in abs.ancestors() {
        if a.file_name().is_some_and(|n| n == "src") {
            return a.to_path_buf();
        }
    }
    abs
}

fn projectless_cache_root(start: &Path) -> PathBuf {
    use sha2::{Digest, Sha256};
    let root = projectless_root(start);
    let h = Sha256::digest(root.to_string_lossy().as_bytes());
    let key: String = h.iter().take(8).map(|b| format!("{b:02x}")).collect();
    global_meta_cache_dir().join(key)
}

fn projectless_meta_path(source_file: &Path) -> PathBuf {
    let root = projectless_root(source_file);
    let abs = source_file.canonicalize().unwrap_or_else(|_| {
        if source_file.is_absolute() { source_file.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(source_file) }
    });
    let rel = abs.strip_prefix(&root).map(Path::to_path_buf).unwrap_or_else(|_| PathBuf::from(abs.file_name().unwrap_or_default()));
    let mut p = projectless_cache_root(source_file);
    p.push(rel);
    let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
    p.set_file_name(format!("{name}.meta"));
    p
}

/// True when `path` is inside an analysis cache (a `.wj-cache` directory or the global meta cache).
/// A `*.wj.meta` anywhere else is a legacy sidecar and is never read.
pub fn is_cache_location(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".wj-cache") || path.starts_with(global_meta_cache_dir())
}

/// Get the `.wj-cache/` root for a given source root.
///
/// Finds the project root (nearest `Cargo.toml`/`wj.toml`) and returns `<project>/.wj-cache/`.
/// Without a project marker it is a per-tree directory in the global meta cache (never inside the source tree).
pub fn meta_cache_root(source_root: &Path) -> PathBuf {
    if let Some(project_root) = find_project_root(source_root) {
        project_root.join(".wj-cache")
    } else {
        projectless_cache_root(source_root)
    }
}

/// Crate-level metadata for cross-crate type inference.
/// Emitted as metadata.json when building libraries (--library).
/// Loaded when compiling apps that depend on external Windjammer crates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateMetadata {
    /// All structs: struct_name → field_name → serialized Type
    pub structs: HashMap<String, HashMap<String, String>>,
    /// All function signatures: name → signature
    pub functions: HashMap<String, FunctionSignature>,
    /// Structs that implement Copy (enables cross-crate Copy detection)
    #[serde(default)]
    pub copy_structs: Vec<String>,
    /// `const string` names that lower to `&'static str` (cross-crate owned formals).
    #[serde(default)]
    pub string_consts: Vec<String>,
    /// Version for compatibility
    pub version: String,
}

impl Default for CrateMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl CrateMetadata {
    pub fn new() -> Self {
        CrateMetadata {
            structs: HashMap::new(),
            functions: HashMap::new(),
            copy_structs: Vec::new(),
            string_consts: Vec::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Merge in struct/function data from a ModuleMetadata
    pub fn merge_module(&mut self, module: &ModuleMetadata) {
        for (struct_name, fields) in &module.structs {
            self.structs
                .entry(struct_name.clone())
                .or_default()
                .extend(fields.clone());
        }
        for (func_name, sig) in &module.functions {
            self.functions.insert(func_name.clone(), sig.clone());
        }
    }
}

/// Extract typed struct field types from a CrateMetadata for cross-crate type inference.
/// Returns struct_name → field_name → Type, enabling `infer_type_name` to resolve
/// nested field access chains like `self.renderer.voxel_renderer` across crate boundaries.
pub fn extract_struct_field_types(
    crate_meta: &CrateMetadata,
) -> HashMap<String, HashMap<String, Type>> {
    let mut result = HashMap::new();
    for (struct_name, fields) in &crate_meta.structs {
        let mut typed_fields = HashMap::new();
        for (field_name, type_str) in fields {
            if let Some(ty) = ModuleMetadata::deserialize_type(type_str) {
                typed_fields.insert(field_name.clone(), ty);
            }
        }
        if !typed_fields.is_empty() {
            result.insert(struct_name.clone(), typed_fields);
        }
    }
    result
}

/// Load struct field types from a metadata.json file on disk.
pub fn load_struct_field_types_from_file(path: &Path) -> HashMap<String, HashMap<String, Type>> {
    let Some(metadata_path) = resolve_metadata_json_path(path) else {
        return HashMap::new();
    };
    let Ok(text) = std::fs::read_to_string(&metadata_path) else {
        return HashMap::new();
    };
    let Ok(crate_meta) = serde_json::from_str::<CrateMetadata>(&text) else {
        return HashMap::new();
    };
    extract_struct_field_types(&crate_meta)
}

/// Load and merge external struct field types from dependency metadata files.
/// Shared by both `compilation_pipeline` and `library_multipass` to avoid
/// duplicated metadata loading loops.
pub fn load_merged_external_struct_fields(
    external_paths: &HashMap<String, std::path::PathBuf>,
    exclude_local: Option<&std::collections::HashSet<String>>,
) -> HashMap<String, HashMap<String, Type>> {
    let mut merged: HashMap<String, HashMap<String, Type>> = HashMap::new();
    for meta_path in external_paths.values() {
        let fields = load_struct_field_types_from_file(meta_path);
        for (struct_name, field_map) in fields {
            if exclude_local.is_none_or(|locals| !locals.contains(&struct_name)) {
                merged.entry(struct_name).or_default().extend(field_map);
            }
        }
    }
    merged
}

/// Resolve `metadata.json` whether `path` is the file itself or its parent directory.
pub fn resolve_metadata_json_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    let joined = path.join("metadata.json");
    if joined.is_file() {
        Some(joined)
    } else {
        None
    }
}

/// `const string` literal names from a crate `metadata.json` (file or directory).
pub fn load_string_const_names(meta_dir_or_file: &Path) -> Vec<String> {
    let Some(metadata_path) = resolve_metadata_json_path(meta_dir_or_file) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(&metadata_path) else {
        return Vec::new();
    };
    let Ok(crate_meta) = serde_json::from_str::<CrateMetadata>(&text) else {
        return Vec::new();
    };
    crate_meta.string_consts
}

/// Names of `const`/`static` string literals in `program` (Rust `&'static str`).
pub fn collect_string_const_names(program: &crate::parser::Program) -> Vec<String> {
    fn walk(item: &crate::parser::Item, out: &mut Vec<String>) {
        match item {
            crate::parser::Item::Const {
                name, type_, value, ..
            }
            | crate::parser::Item::Static {
                name, type_, value, ..
            } => {
                if crate::codegen::rust::types::is_windjammer_text_type(type_)
                    && matches!(
                        value,
                        crate::parser::Expression::Literal {
                            value: crate::parser::Literal::String(_),
                            ..
                        }
                    )
                {
                    out.push(name.clone());
                }
            }
            crate::parser::Item::Mod { items, .. } => {
                for sub in items {
                    walk(sub, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for item in &program.items {
        walk(item, &mut out);
    }
    out
}

/// Load function signatures from a crate `metadata.json` (file or directory).
/// Shared by float/int numeric inference for cross-crate literal suffixing.
pub fn load_function_signatures_from_metadata(
    meta_dir_or_file: &Path,
) -> HashMap<String, (Vec<Type>, Option<Type>)> {
    let Some(metadata_path) = resolve_metadata_json_path(meta_dir_or_file) else {
        return HashMap::new();
    };
    let Ok(text) = std::fs::read_to_string(&metadata_path) else {
        return HashMap::new();
    };
    let Ok(crate_meta) = serde_json::from_str::<CrateMetadata>(&text) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for (func_name, sig) in &crate_meta.functions {
        let params: Vec<Type> = sig
            .params
            .iter()
            .filter_map(|s| ModuleMetadata::deserialize_type(s))
            .collect();
        let return_type = sig
            .return_type
            .as_ref()
            .and_then(|s| ModuleMetadata::deserialize_type(s));
        out.insert(func_name.clone(), (params, return_type));
    }
    out
}

/// Merge function signatures from all external crate metadata roots.
pub fn load_merged_external_function_signatures(
    external_paths: &HashMap<String, PathBuf>,
) -> HashMap<String, (Vec<Type>, Option<Type>)> {
    let mut merged = HashMap::new();
    for meta_dir in external_paths.values() {
        for (name, sig) in load_function_signatures_from_metadata(meta_dir) {
            merged.insert(name, sig);
        }
    }
    merged
}

pub(in crate::metadata) fn merge_crate_metadata_file(
    path: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    copy_structs: &mut Vec<String>,
    all_struct_fields: &mut HashMap<String, Vec<Vec<String>>>,
) {
    merge_crate_metadata_file_with_alias(path, registry, copy_structs, all_struct_fields, None);
}

/// Like [`merge_crate_metadata_file`], and when `crate_alias` is set also registers
// `alias::fn` keys for each bare function (qualified external calls).
pub(in crate::metadata) fn merge_crate_metadata_file_with_alias(
    path: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    copy_structs: &mut Vec<String>,
    all_struct_fields: &mut HashMap<String, Vec<Vec<String>>>,
    crate_alias: Option<&str>,
) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(crate_meta) = serde_json::from_str::<CrateMetadata>(&text) else {
        return;
    };
    copy_structs.extend(crate_meta.copy_structs.iter().cloned());
    for (struct_name, fields) in &crate_meta.structs {
        let field_types: Vec<String> = fields.iter().map(|(_, ty)| ty.clone()).collect();
        all_struct_fields
            .entry(struct_name.clone())
            .or_default()
            .push(field_types);
    }
    for (name, sig) in &crate_meta.functions {
        let added = if let Some(a_sig) = try_analyzer_signature_from_metadata(name, sig) {
            if sig.is_trait_method {
                registry.record_trait_method_key(name.clone());
            }
            // Path-dep / `--metadata` crates: register `crate_key::fn` only.
            // Bare `query_get` from `borrowed_pkg` steals `use owned_pkg::get as query_get`
            // (P3.283 / P3.448). Local crates (no alias) still install the bare key.
            if crate_alias.is_none() || name.contains("::") {
                registry.add_function(name.clone(), a_sig.clone());
            }
            Some(a_sig)
        } else if sig.is_extern {
            // Extern functions with no param_ownership still need registry entries
            // so the codegen can wrap calls in unsafe blocks.
            let param_types: Vec<Type> = sig
                .params
                .iter()
                .filter_map(|p| ModuleMetadata::deserialize_type(p))
                .collect();
            let param_ownership = param_types
                .iter()
                .map(|_| crate::analyzer::OwnershipMode::Owned)
                .collect();
            let formal_param_types = param_types.clone();
            let return_type = sig
                .return_type
                .as_ref()
                .and_then(|s| ModuleMetadata::deserialize_type(s));
            let a_sig = AnalyzerFunctionSignature {
                name: name.to_string(),
                param_types,
                formal_param_types,
                param_ownership,
                return_type,
                return_ownership: OwnershipMode::Owned,
                has_self_receiver: sig.has_self_receiver,
                is_extern: true,
                emitted_rust_ref_params: None,
                string_ref_string_formal_params: None,
                field_extract_params: None,
                forwarding_borrow_params: None,
            };
            if crate_alias.is_none() || name.contains("::") {
                registry.add_function(name.clone(), a_sig.clone());
            }
            Some(a_sig)
        } else {
            None
        };
        if let (Some(alias), Some(a_sig)) = (crate_alias, added) {
            if !name.contains("::") && !alias.is_empty() {
                let qualified = format!("{alias}::{name}");
                if registry.get_signature(&qualified).is_none() {
                    registry.add_function(qualified, a_sig);
                }
            }
        }
    }
}
