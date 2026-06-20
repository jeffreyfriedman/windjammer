//! Crate-level metadata (`metadata.json`) and project cache path resolution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::analyzer::{FunctionSignature as AnalyzerFunctionSignature, OwnershipMode};
use crate::parser::Type;

use super::function_metadata::{try_analyzer_signature_from_metadata, FunctionSignature};
use super::ModuleMetadata;

/// Find the project root by walking up from `start` looking for `Cargo.toml` or `wj.toml`.
pub(in crate::metadata) fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?
    } else {
        start
    };
    loop {
        if dir.join("Cargo.toml").exists() || dir.join("wj.toml").exists() {
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
            let mut cache_path = project_root.join(".wj-cache");
            cache_path.push(relative);
            let file_name = cache_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            cache_path.set_file_name(format!("{}.meta", file_name));
            return cache_path;
        }
        // File is in the project but not under src/ -- place relative to project root
        if let Ok(relative) = source_file.strip_prefix(&project_root) {
            let mut cache_path = project_root.join(".wj-cache");
            cache_path.push(relative);
            let file_name = cache_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            cache_path.set_file_name(format!("{}.meta", file_name));
            return cache_path;
        }
    }
    source_file.with_extension("wj.meta")
}

/// Get the `.wj-cache/` root for a given source root.
///
/// Finds the project root (nearest `Cargo.toml`/`wj.toml`) and returns `<project>/.wj-cache/`.
/// Falls back to placing `.wj-cache/` inside the given root if no project marker is found.
pub fn meta_cache_root(source_root: &Path) -> PathBuf {
    if let Some(project_root) = find_project_root(source_root) {
        project_root.join(".wj-cache")
    } else {
        source_root.join(".wj-cache")
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
    let Ok(text) = std::fs::read_to_string(path) else {
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

pub(in crate::metadata) fn merge_crate_metadata_file(
    path: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    _copy_structs: &mut Vec<String>,
    _all_struct_fields: &mut HashMap<String, Vec<Vec<String>>>,
) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(crate_meta) = serde_json::from_str::<CrateMetadata>(&text) else {
        return;
    };
    for (name, sig) in &crate_meta.functions {
        if let Some(a_sig) = try_analyzer_signature_from_metadata(name, sig) {
            registry.add_function(name.clone(), a_sig);
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
            registry.add_function(
                name.clone(),
                AnalyzerFunctionSignature {
                    name: name.to_string(),
                    param_types,
                    formal_param_types,
                    param_ownership,
                    return_type,
                    return_ownership: OwnershipMode::Owned,
                    has_self_receiver: sig.has_self_receiver,
                    is_extern: true,
                },
            );
        }
    }
}
