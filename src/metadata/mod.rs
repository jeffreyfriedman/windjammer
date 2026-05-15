/// Metadata System for Cross-Module Type Inference
///
/// Enables type inference across file boundaries by emitting and loading
/// function signatures, struct fields, and trait implementations.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

mod crate_metadata;
mod function_metadata;
mod type_metadata;

pub use crate_metadata::{meta_cache_path, meta_cache_root, CrateMetadata};
pub use function_metadata::{
    metadata_function_sig_from_analyzer, try_analyzer_signature_from_metadata, FunctionSignature,
};
pub use type_metadata::infer_copy_from_metadata_structs_pub;

/// Metadata for a single Windjammer module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    /// Module path (e.g., "math::vec3")
    pub module_path: String,

    /// Function signatures: name → (param_types, return_type)
    pub functions: HashMap<String, FunctionSignature>,

    /// Struct field types: struct_name → field_name → Type
    pub structs: HashMap<String, HashMap<String, String>>, // String = serialized Type

    /// Trait implementations: trait_name → methods
    pub trait_impls: HashMap<String, Vec<String>>,

    /// Structs that implement Copy (enables cross-file Copy detection)
    #[serde(default)]
    pub copy_structs: Vec<String>,

    /// Version for compatibility checking
    pub version: String,
}

impl ModuleMetadata {
    pub fn new(module_path: String) -> Self {
        ModuleMetadata {
            module_path,
            functions: HashMap::new(),
            structs: HashMap::new(),
            trait_impls: HashMap::new(),
            copy_structs: Vec::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Recursively load `*.wj.meta` under `root` and merge function signatures into the registry.
/// Also collects Copy struct names from metadata for cross-file Copy detection.
/// Searches both `root` (legacy colocated meta) and the `.wj-cache/` sibling directory.
pub fn merge_wj_meta_signatures_from_dir(
    root: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
) {
    let mut copy_structs = Vec::new();
    let mut all_struct_fields = HashMap::new();
    let cache_root = crate_metadata::meta_cache_root(root);
    if cache_root.exists() {
        merge_wj_meta_signatures_from_dir_inner(
            &cache_root,
            registry,
            &mut copy_structs,
            &mut all_struct_fields,
        );
    }
    merge_wj_meta_signatures_from_dir_inner(
        root,
        registry,
        &mut copy_structs,
        &mut all_struct_fields,
    );
    type_metadata::infer_copy_from_metadata_structs(&all_struct_fields, &mut copy_structs);
}

/// Same as `merge_wj_meta_signatures_from_dir` but also populates the analyzer with Copy struct info.
pub fn merge_wj_meta_signatures_and_copy_structs(
    root: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    analyzer: &mut crate::analyzer::Analyzer,
) {
    let mut copy_structs = Vec::new();
    let mut all_struct_fields: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let cache_root = crate_metadata::meta_cache_root(root);
    if cache_root.exists() {
        merge_wj_meta_signatures_from_dir_inner(
            &cache_root,
            registry,
            &mut copy_structs,
            &mut all_struct_fields,
        );
    }
    merge_wj_meta_signatures_from_dir_inner(
        root,
        registry,
        &mut copy_structs,
        &mut all_struct_fields,
    );
    type_metadata::infer_copy_from_metadata_structs(&all_struct_fields, &mut copy_structs);
    for name in &copy_structs {
        analyzer.register_copy_struct(name);
    }
}

/// Load metadata from multiple root directories, merging all results.
/// Used when cross-crate dependencies need to be resolved.
pub fn merge_wj_meta_signatures_and_copy_structs_multi(
    roots: &[&Path],
    registry: &mut crate::analyzer::SignatureRegistry,
    analyzer: &mut crate::analyzer::Analyzer,
) {
    let mut copy_structs = Vec::new();
    let mut all_struct_fields: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for root in roots {
        let cache_root = crate_metadata::meta_cache_root(root);
        if cache_root.exists() {
            merge_wj_meta_signatures_from_dir_inner(
                &cache_root,
                registry,
                &mut copy_structs,
                &mut all_struct_fields,
            );
        }
        merge_wj_meta_signatures_from_dir_inner(
            root,
            registry,
            &mut copy_structs,
            &mut all_struct_fields,
        );
    }
    type_metadata::infer_copy_from_metadata_structs(&all_struct_fields, &mut copy_structs);
    for name in &copy_structs {
        analyzer.register_copy_struct(name);
    }
}

/// Public accessor for `merge_wj_meta_signatures_from_dir_inner` (used by compiler multipass).
/// Scans both the cache directory and the source root for `.wj.meta` files.
pub fn merge_wj_meta_signatures_from_dir_inner_pub(
    root: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    copy_structs: &mut Vec<String>,
    all_struct_fields: &mut HashMap<String, Vec<Vec<String>>>,
) {
    let cache_root = crate_metadata::meta_cache_root(root);
    if cache_root.exists() {
        merge_wj_meta_signatures_from_dir_inner(
            &cache_root,
            registry,
            copy_structs,
            all_struct_fields,
        );
    }
    merge_wj_meta_signatures_from_dir_inner(root, registry, copy_structs, all_struct_fields);
}

fn merge_wj_meta_signatures_from_dir_inner(
    root: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    copy_structs: &mut Vec<String>,
    all_struct_fields: &mut HashMap<String, Vec<Vec<String>>>,
) {
    // If root is a file (e.g. metadata.json passed directly), handle it
    if root.is_file() {
        if let Some(name) = root.file_name().and_then(|n| n.to_str()) {
            if name == "metadata.json" {
                crate_metadata::merge_crate_metadata_file(
                    root,
                    registry,
                    copy_structs,
                    all_struct_fields,
                );
            } else if name.ends_with(".wj.meta") {
                merge_single_wj_meta_file(root, registry, copy_structs, all_struct_fields);
            }
        }
        return;
    }

    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            merge_wj_meta_signatures_from_dir_inner(
                &path,
                registry,
                copy_structs,
                all_struct_fields,
            );
        } else if path.to_string_lossy().ends_with(".wj.meta") {
            merge_single_wj_meta_file(&path, registry, copy_structs, all_struct_fields);
        } else if path
            .file_name()
            .map(|n| n == "metadata.json")
            .unwrap_or(false)
        {
            crate_metadata::merge_crate_metadata_file(
                &path,
                registry,
                copy_structs,
                all_struct_fields,
            );
        }
    }
}

fn merge_single_wj_meta_file(
    path: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    copy_structs: &mut Vec<String>,
    all_struct_fields: &mut HashMap<String, Vec<Vec<String>>>,
) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(mod_meta) = serde_json::from_str::<ModuleMetadata>(&text) else {
        return;
    };
    function_metadata::merge_module_metadata_signatures(&mod_meta, registry);
    copy_structs.extend(mod_meta.copy_structs.iter().cloned());
    for (struct_name, fields) in &mod_meta.structs {
        let field_types: Vec<String> = fields.values().cloned().collect();
        all_struct_fields
            .entry(struct_name.clone())
            .or_default()
            .push(field_types);
    }
}

#[cfg(test)]
mod tests;
