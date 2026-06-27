/// Metadata System for Cross-Module Type Inference
///
/// Enables type inference across file boundaries by emitting and loading
/// function signatures, struct fields, and trait implementations.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

mod crate_metadata;
mod function_metadata;
mod signature_filters;
mod type_metadata;

pub use signature_filters::{
    drop_dependency_signatures_for_local_types, signature_targets_local_struct,
    struct_name_from_method_key,
};

pub use crate_metadata::{
    load_merged_external_struct_fields, load_struct_field_types_from_file, meta_cache_path,
    meta_cache_root, CrateMetadata,
};
pub use function_metadata::{
    default_skeleton_param_ownership_from_types, metadata_function_sig_from_analyzer,
    try_analyzer_signature_from_metadata, FunctionSignature,
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

    /// Structs that explicitly opted out of Copy via @derive() without Copy.
    /// Prevents metadata inference from falsely marking them as Copy.
    #[serde(default)]
    pub non_copy_structs: Vec<String>,

    /// Version for compatibility checking
    pub version: String,

    /// Content + compiler fingerprint for incremental analysis cache validation.
    #[serde(default)]
    pub analysis_fingerprint: Option<AnalysisFingerprint>,
}

/// Fingerprint stored in `.wj.meta` to skip re-analysis when source unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalysisFingerprint {
    pub content_hash: u64,
    pub compiler_version: String,
    pub dep_metadata_epoch: u64,
    /// Hash of generated `.rs` at last successful codegen (see build_fingerprint).
    #[serde(default)]
    pub output_hash: u64,
}

impl ModuleMetadata {
    pub fn new(module_path: String) -> Self {
        ModuleMetadata {
            module_path,
            functions: HashMap::new(),
            structs: HashMap::new(),
            trait_impls: HashMap::new(),
            copy_structs: Vec::new(),
            non_copy_structs: Vec::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            analysis_fingerprint: None,
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
    load_project_root_metadata(root, registry, &mut copy_structs, &mut all_struct_fields);
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
    load_project_root_metadata(root, registry, &mut copy_structs, &mut all_struct_fields);
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
    // Project-root metadata.json is the authoritative source (full multi-pass
    // analysis). Load it last so it overrides stale per-file .wj.meta entries.
    for root in roots {
        load_project_root_metadata(root, registry, &mut copy_structs, &mut all_struct_fields);
    }
    type_metadata::infer_copy_from_metadata_structs(&all_struct_fields, &mut copy_structs);
    for name in &copy_structs {
        analyzer.register_copy_struct(name);
    }
}

/// Public accessor for `merge_wj_meta_signatures_from_dir_inner` (used by compiler multipass).
/// Scans both the cache directory and the source root for `.wj.meta` files.
/// Loads the project-root `metadata.json` last as authoritative source.
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
    load_project_root_metadata(root, registry, copy_structs, all_struct_fields);
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

/// Load the project-root `metadata.json` as the final authoritative step.
/// This ensures crate-level metadata (from a full multi-pass analysis) overrides
/// any stale per-file `.wj.meta` entries that may have incorrect ownership inference.
fn load_project_root_metadata(
    root: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    copy_structs: &mut Vec<String>,
    all_struct_fields: &mut HashMap<String, Vec<Vec<String>>>,
) {
    if let Some(project_root) = crate_metadata::find_project_root(root) {
        let meta_path = project_root.join("metadata.json");
        if meta_path.exists() {
            crate_metadata::merge_crate_metadata_file(
                &meta_path,
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
    let meta_copy_set: HashSet<&str> = mod_meta.copy_structs.iter().map(|s| s.as_str()).collect();
    copy_structs.extend(mod_meta.copy_structs.iter().cloned());
    for (struct_name, fields) in &mod_meta.structs {
        // Only add field info for structs the upstream crate marked as Copy.
        // Types that opted out (e.g., @derive(Debug, Clone) without Copy, or
        // types with Drop impls) must not be re-inferred as Copy based solely
        // on their field types.
        if meta_copy_set.contains(struct_name.as_str()) {
            let field_types: Vec<String> = fields.values().cloned().collect();
            all_struct_fields
                .entry(struct_name.clone())
                .or_default()
                .push(field_types);
        }
    }
}

/// Build pre-analysis metadata from AST items only (no registry needed).
/// Used during library compilation to seed `CrateMetadata` before ownership analysis.
pub fn collect_ast_skeleton_metadata(program: &crate::parser::Program) -> ModuleMetadata {
    use crate::parser::ast::core::Item;

    let mut meta = ModuleMetadata::new(String::new());
    for item in &program.items {
        match item {
            Item::Struct { decl, .. } => {
                let mut fields = HashMap::new();
                for field in &decl.fields {
                    fields.insert(
                        field.name.clone(),
                        ModuleMetadata::serialize_type(&field.field_type),
                    );
                }
                meta.structs.insert(decl.name.clone(), fields);
            }
            Item::Function { decl, .. } => {
                let param_types: Vec<_> = decl.parameters.iter().map(|p| p.type_.clone()).collect();
                meta.functions.insert(
                    decl.name.clone(),
                    FunctionSignature {
                        params: param_types
                            .iter()
                            .map(ModuleMetadata::serialize_type)
                            .collect(),
                        formal_params: param_types
                            .iter()
                            .map(ModuleMetadata::serialize_type)
                            .collect(),
                        return_type: decl
                            .return_type
                            .as_ref()
                            .map(ModuleMetadata::serialize_type),
                        is_associated: false,
                        parent_type: None,
                        param_ownership: default_skeleton_param_ownership_from_types(&param_types),
                        has_self_receiver: false,
                        is_extern: decl.is_extern,
                    },
                );
            }
            Item::Impl { block, .. } => {
                for func_decl in &block.functions {
                    let param_types: Vec<_> = func_decl
                        .parameters
                        .iter()
                        .map(|p| p.type_.clone())
                        .collect();
                    let full_name = format!("{}::{}", block.type_name, func_decl.name);
                    meta.functions.insert(
                        full_name,
                        FunctionSignature {
                            params: param_types
                                .iter()
                                .map(ModuleMetadata::serialize_type)
                                .collect(),
                            formal_params: param_types
                                .iter()
                                .map(ModuleMetadata::serialize_type)
                                .collect(),
                            return_type: func_decl
                                .return_type
                                .as_ref()
                                .map(ModuleMetadata::serialize_type),
                            is_associated: true,
                            parent_type: Some(block.type_name.clone()),
                            param_ownership: default_skeleton_param_ownership_from_types(
                                &param_types,
                            ),
                            has_self_receiver: false,
                            is_extern: false,
                        },
                    );
                }
            }
            _ => {}
        }
    }
    meta
}

/// Collect pre-analysis metadata from the AST and merge it into a CrateMetadata.
/// Derives the `module_path` from the file stem.
pub fn merge_file_skeleton_into_crate(
    crate_metadata: &mut CrateMetadata,
    file: &std::path::Path,
    program: &crate::parser::Program,
) {
    let mut module_meta = collect_ast_skeleton_metadata(program);
    module_meta.module_path = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    crate_metadata.merge_module(&module_meta);
}

/// Build a ModuleMetadata from analyzed program items and a converged signature registry.
/// This extracts function/impl/struct/trait metadata from the AST using the registry.
/// Used by both single-file and multipass compilation pipelines.
pub fn collect_analyzed_module_metadata(
    module_name: &str,
    program: &crate::parser::Program,
    registry: &crate::analyzer::SignatureRegistry,
    copy_structs: Vec<String>,
) -> ModuleMetadata {
    use crate::parser::ast::core::Item;

    let mut meta = ModuleMetadata::new(module_name.to_string());
    for item in &program.items {
        match item {
            Item::Function { decl, .. } => {
                let sig = registry.get_signature(&decl.name).or_else(|| {
                    if module_name.is_empty() {
                        None
                    } else {
                        registry.get_signature(&format!("{}::{}", module_name, decl.name))
                    }
                });
                if let Some(sig) = sig {
                    let key = if registry.get_signature(&decl.name).is_some() {
                        decl.name.clone()
                    } else {
                        format!("{}::{}", module_name, decl.name)
                    };
                    meta.functions
                        .insert(key, metadata_function_sig_from_analyzer(sig, false, None));
                }
            }
            Item::Impl { block, .. } => {
                for func_decl in &block.functions {
                    let full_name = format!("{}::{}", block.type_name, func_decl.name);
                    if let Some(sig) = registry.get_signature(&full_name) {
                        meta.functions.insert(
                            full_name,
                            metadata_function_sig_from_analyzer(
                                sig,
                                true,
                                Some(block.type_name.clone()),
                            ),
                        );
                    }
                }
            }
            Item::Struct { decl, .. } => {
                let mut fields = HashMap::new();
                for field in &decl.fields {
                    fields.insert(
                        field.name.clone(),
                        ModuleMetadata::serialize_type(&field.field_type),
                    );
                }
                meta.structs.insert(decl.name.clone(), fields);
            }
            Item::Trait { decl, .. } => {
                for method in &decl.methods {
                    let full_name = format!("{}::{}", decl.name, method.name);
                    if let Some(sig) = registry.get_signature(&full_name) {
                        meta.functions.insert(
                            full_name,
                            metadata_function_sig_from_analyzer(sig, true, Some(decl.name.clone())),
                        );
                    }
                }
            }
            _ => {}
        }
    }
    meta.copy_structs = copy_structs;

    // Collect structs that explicitly opted out of Copy via @derive() without Copy
    for item in &program.items {
        if let Item::Struct { decl, .. } = item {
            let has_derive_without_copy = decl.decorators.iter().any(|d| {
                d.name == "derive"
                    && !d.arguments.iter().any(|(_, arg)| {
                        matches!(arg, crate::parser::Expression::Identifier { name, .. } if name == "Copy")
                    })
            });
            if has_derive_without_copy {
                meta.non_copy_structs.push(decl.name.clone());
            }
        }
    }

    meta
}

/// Write a ModuleMetadata to the .wj.meta cache file for the given source path.
pub fn write_module_meta_cache(source_file: &Path, meta: &ModuleMetadata) -> std::io::Result<()> {
    let meta_path = meta_cache_path(source_file);
    if let Some(parent) = meta_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(meta) {
        crate::compiler::write_if_changed(&meta_path, &json)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
    }
    Ok(())
}

/// Emit per-file `.wj.meta` for a compiled module.
/// Shared by both `compilation_pipeline` and `library_multipass`.
pub fn emit_module_meta_for_file(
    source_file: &Path,
    program: &crate::parser::Program,
    registry: &crate::analyzer::SignatureRegistry,
    copy_structs: Vec<String>,
) {
    emit_module_meta_for_file_with_fingerprint(source_file, program, registry, copy_structs, None);
}

pub fn emit_module_meta_for_file_with_fingerprint(
    source_file: &Path,
    program: &crate::parser::Program,
    registry: &crate::analyzer::SignatureRegistry,
    copy_structs: Vec<String>,
    analysis_fingerprint: Option<AnalysisFingerprint>,
) {
    let module_name = source_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let mut meta = collect_analyzed_module_metadata(module_name, program, registry, copy_structs);
    meta.analysis_fingerprint = analysis_fingerprint;
    let _ = write_module_meta_cache(source_file, &meta);
}

#[cfg(test)]
mod tests;
