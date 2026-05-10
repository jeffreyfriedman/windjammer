/// Metadata System for Cross-Module Type Inference
///
/// Enables type inference across file boundaries by emitting and loading
/// function signatures, struct fields, and trait implementations.
use crate::analyzer::{FunctionSignature as AnalyzerFunctionSignature, OwnershipMode};
use crate::parser::ast::types::Type;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Find the project root by walking up from `start` looking for `Cargo.toml` or `wj.toml`.
fn find_project_root(start: &Path) -> Option<PathBuf> {
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

/// Function signature for type inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Parameter types
    pub params: Vec<String>, // Serialized Type (JSON is easier than bincode for now)

    /// Return type (None for unit)
    pub return_type: Option<String>,

    /// Is this an associated function (Type::method)?
    pub is_associated: bool,

    /// Parent type for associated functions (e.g., "Vec3" for Vec3::new)
    pub parent_type: Option<String>,

    /// Inferred ownership modes for each parameter (Owned, Borrowed, MutBorrowed), Debug string form
    #[serde(default)]
    pub param_ownership: Vec<String>,

    /// True when the signature includes a `self` receiver (matches analyzer `has_self_receiver`)
    #[serde(default)]
    pub has_self_receiver: bool,

    /// True when this is an `extern fn` (FFI). Used by codegen to wrap calls in `unsafe`.
    #[serde(default)]
    pub is_extern: bool,
}

/// Build metadata JSON signature from an analyzer registry entry (post-inference).
pub fn metadata_function_sig_from_analyzer(
    sig: &AnalyzerFunctionSignature,
    is_associated: bool,
    parent_type: Option<String>,
) -> FunctionSignature {
    FunctionSignature {
        params: sig
            .param_types
            .iter()
            .map(ModuleMetadata::serialize_type)
            .collect(),
        return_type: sig.return_type.as_ref().map(ModuleMetadata::serialize_type),
        is_associated,
        parent_type,
        param_ownership: sig
            .param_ownership
            .iter()
            .map(|o| format!("{:?}", o))
            .collect(),
        has_self_receiver: sig.has_self_receiver,
        is_extern: sig.is_extern,
    }
}

fn ownership_mode_from_meta_str(s: &str) -> Option<OwnershipMode> {
    match s {
        "Owned" => Some(OwnershipMode::Owned),
        "Borrowed" => Some(OwnershipMode::Borrowed),
        "MutBorrowed" => Some(OwnershipMode::MutBorrowed),
        _ => None,
    }
}

/// Convert loaded metadata to an analyzer signature when `param_ownership` is present (new format).
pub fn try_analyzer_signature_from_metadata(
    name: &str,
    meta_sig: &FunctionSignature,
) -> Option<AnalyzerFunctionSignature> {
    if meta_sig.param_ownership.is_empty() {
        return None;
    }
    let mut param_types = Vec::with_capacity(meta_sig.params.len());
    for p in &meta_sig.params {
        let ty = ModuleMetadata::deserialize_type(p)?;
        param_types.push(ty);
    }
    if param_types.len() != meta_sig.param_ownership.len() {
        return None;
    }
    let mut param_ownership = Vec::with_capacity(meta_sig.param_ownership.len());
    for o in &meta_sig.param_ownership {
        param_ownership.push(ownership_mode_from_meta_str(o)?);
    }
    let return_type = meta_sig
        .return_type
        .as_ref()
        .and_then(|s| ModuleMetadata::deserialize_type(s));

    Some(AnalyzerFunctionSignature {
        name: name.to_string(),
        param_types,
        param_ownership,
        return_type,
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: meta_sig.has_self_receiver,
        is_extern: meta_sig.is_extern,
    })
}

fn merge_module_metadata_signatures(
    meta: &ModuleMetadata,
    registry: &mut crate::analyzer::SignatureRegistry,
) {
    for (name, sig) in &meta.functions {
        if let Some(a_sig) = try_analyzer_signature_from_metadata(name, sig) {
            registry.add_function(name.clone(), a_sig.clone());
            if !meta.module_path.is_empty() && !name.contains("::") {
                let qualified = format!("{}::{}", meta.module_path, name);
                registry.add_function(qualified, a_sig);
            }
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
    let cache_root = meta_cache_root(root);
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
    infer_copy_from_metadata_structs(&all_struct_fields, &mut copy_structs);
}

/// Same as `merge_wj_meta_signatures_from_dir` but also populates the analyzer with Copy struct info.
pub fn merge_wj_meta_signatures_and_copy_structs(
    root: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    analyzer: &mut crate::analyzer::Analyzer,
) {
    let mut copy_structs = Vec::new();
    let mut all_struct_fields: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let cache_root = meta_cache_root(root);
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
    infer_copy_from_metadata_structs(&all_struct_fields, &mut copy_structs);
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
        let cache_root = meta_cache_root(root);
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
    infer_copy_from_metadata_structs(&all_struct_fields, &mut copy_structs);
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
    let cache_root = meta_cache_root(root);
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

/// Public accessor for `infer_copy_from_metadata_structs` (used by compiler multipass).
pub fn infer_copy_from_metadata_structs_pub(
    all_struct_fields: &HashMap<String, Vec<Vec<String>>>,
    existing_copy: &mut Vec<String>,
) {
    infer_copy_from_metadata_structs(all_struct_fields, existing_copy);
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
                merge_crate_metadata_file(root, registry, copy_structs, all_struct_fields);
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
        } else if path.file_name().map(|n| n == "metadata.json").unwrap_or(false) {
            merge_crate_metadata_file(&path, registry, copy_structs, all_struct_fields);
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
    merge_module_metadata_signatures(&mod_meta, registry);
    copy_structs.extend(mod_meta.copy_structs.iter().cloned());
    for (struct_name, fields) in &mod_meta.structs {
        let field_types: Vec<String> = fields.values().cloned().collect();
        all_struct_fields
            .entry(struct_name.clone())
            .or_default()
            .push(field_types);
    }
}

fn merge_crate_metadata_file(
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
            let param_types: Vec<crate::parser::Type> = sig
                .params
                .iter()
                .filter_map(|p| ModuleMetadata::deserialize_type(p))
                .collect();
            let param_ownership = param_types
                .iter()
                .map(|_| crate::analyzer::OwnershipMode::Owned)
                .collect();
            let return_type = sig
                .return_type
                .as_ref()
                .and_then(|s| ModuleMetadata::deserialize_type(s));
            registry.add_function(
                name.clone(),
                AnalyzerFunctionSignature {
                    name: name.to_string(),
                    param_types,
                    param_ownership,
                    return_type,
                    return_ownership: crate::analyzer::OwnershipMode::Owned,
                    has_self_receiver: sig.has_self_receiver,
                    is_extern: true,
                },
            );
        }
    }
}

/// Infer Copy types from struct field definitions loaded from metadata.
/// A struct is Copy if all its fields are known Copy types.
/// Uses fixpoint iteration to handle transitive Copy (e.g., struct A { b: B } where B is Copy).
///
/// TDD FIX: Conservative handling of duplicate struct names across modules.
/// If multiple metadata files define structs with the same name, only mark as Copy
/// if ALL variants are Copy. This prevents one Copy-able GameState from poisoning
/// a non-Copy GameState in a different module.
fn infer_copy_from_metadata_structs(
    all_struct_fields: &HashMap<String, Vec<Vec<String>>>,
    existing_copy: &mut Vec<String>,
) {
    use std::collections::HashSet;
    let mut copy_set: HashSet<String> = existing_copy.iter().cloned().collect();

    const MAX_PASSES: usize = 32;
    for _ in 0..MAX_PASSES {
        let mut changed = false;
        for (struct_name, variants) in all_struct_fields {
            if copy_set.contains(struct_name) {
                continue;
            }

            // TDD FIX: Check if ALL variants are Copy (conservative)
            let all_variants_copy = variants.iter().all(|field_types| {
                field_types
                    .iter()
                    .all(|ft| is_copy_type_string(ft, &copy_set))
            });

            if all_variants_copy {
                copy_set.insert(struct_name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    for name in &copy_set {
        if !existing_copy.contains(name) {
            existing_copy.push(name.clone());
        }
    }
}

/// Check if a serialized Type string represents a Copy type.
fn is_copy_type_string(s: &str, copy_set: &std::collections::HashSet<String>) -> bool {
    match s {
        "Bool" | "Int32" | "Float" => true,
        s if s.starts_with("Custom(\"") && s.ends_with("\")") => {
            let name = &s[8..s.len() - 2];
            matches!(
                name,
                "f32"
                    | "f64"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "isize"
                    | "bool"
                    | "char"
            ) || copy_set.contains(name)
        }
        s if s.starts_with("Array(") => {
            // Array(InnerType, N) - Copy if InnerType is Copy
            let inner = &s[6..s.len() - 1];
            if let Some(comma_pos) = inner.rfind(", ") {
                let ty_str = &inner[..comma_pos];
                is_copy_type_string(ty_str.trim(), copy_set)
            } else {
                false
            }
        }
        _ => false,
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

    /// Serialize Type to JSON string (for metadata storage)
    pub fn serialize_type(ty: &Type) -> String {
        // For MVP: Use Debug format (simple but works)
        // TODO: Proper serde for Type enum
        format!("{:?}", ty)
    }

    /// Deserialize Type from JSON string (Debug format from serialize_type)
    pub fn deserialize_type(s: &str) -> Option<Type> {
        // For MVP: Parse simple types manually
        // TODO: Proper serde for Type enum
        match s {
            "Custom(\"f32\")" => Some(Type::Custom("f32".to_string())),
            "Custom(\"f64\")" => Some(Type::Custom("f64".to_string())),
            "Custom(\"i32\")" => Some(Type::Custom("i32".to_string())),
            "Custom(\"u32\")" => Some(Type::Custom("u32".to_string())),
            "Custom(\"Self\")" => Some(Type::Custom("Self".to_string())),
            "Int32" => Some(Type::Int32),
            "Float" => Some(Type::Float),
            "Bool" => Some(Type::Bool),
            "String" => Some(Type::String),
            s if s.starts_with("Array(") && s.ends_with(')') => {
                // Array(Custom("f32"), 16) or Array(InnerType, N)
                let inner = &s[6..s.len() - 1];
                if let Some(comma_pos) = inner.rfind(", ") {
                    let (ty_str, n_str) = inner.split_at(comma_pos);
                    let n_str = n_str.trim_start_matches(", ");
                    if let (Some(inner_ty), Ok(n)) = (
                        Self::deserialize_type(ty_str.trim()),
                        n_str.parse::<usize>(),
                    ) {
                        return Some(Type::Array(Box::new(inner_ty), n));
                    }
                }
                None
            }
            s if s.starts_with("Vec(") && s.ends_with(')') => {
                let inner = &s[4..s.len() - 1];
                Self::deserialize_type(inner).map(|t| Type::Vec(Box::new(t)))
            }
            s if s.starts_with("Option(") && s.ends_with(')') => {
                let inner = &s[7..s.len() - 1];
                Self::deserialize_type(inner).map(|t| Type::Option(Box::new(t)))
            }
            s if s.starts_with("Custom(") => {
                // Custom("TypeName") - extract the inner string
                let rest = s
                    .strip_prefix("Custom(\"")
                    .and_then(|r| r.strip_suffix("\")"));
                rest.map(|name| Type::Custom(name.to_string()))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_project(base: &std::path::Path, subdirs: &[&str]) {
        std::fs::create_dir_all(base).unwrap();
        std::fs::write(base.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        for sub in subdirs {
            std::fs::create_dir_all(base.join(sub)).unwrap();
        }
    }

    #[test]
    fn test_metadata_round_trip() {
        let mut meta = ModuleMetadata::new("math::vec3".to_string());

        meta.functions.insert(
            "Vec3::new".to_string(),
            FunctionSignature {
                params: vec![
                    "Custom(\"f32\")".to_string(),
                    "Custom(\"f32\")".to_string(),
                    "Custom(\"f32\")".to_string(),
                ],
                return_type: Some("Custom(\"Vec3\")".to_string()),
                is_associated: true,
                parent_type: Some("Vec3".to_string()),
                param_ownership: vec![],
                has_self_receiver: false,
                is_extern: false,
            },
        );

        let json = serde_json::to_string_pretty(&meta).unwrap();
        let loaded: ModuleMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.functions.len(), 1);
        assert!(loaded.functions.contains_key("Vec3::new"));
    }

    #[test]
    fn test_meta_cache_path_src() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("myproject");
        create_project(&proj, &["src/math"]);
        let source = proj.join("src/math/vec3.wj");
        std::fs::write(&source, "").unwrap();

        let result = meta_cache_path(&source);
        assert_eq!(result, proj.join(".wj-cache/math/vec3.wj.meta"));
    }

    #[test]
    fn test_meta_cache_path_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("myproject");
        create_project(&proj, &["src/rendering/shaders"]);
        let source = proj.join("src/rendering/shaders/mesh.wj");
        std::fs::write(&source, "").unwrap();

        let result = meta_cache_path(&source);
        assert_eq!(
            result,
            proj.join(".wj-cache/rendering/shaders/mesh.wj.meta")
        );
    }

    #[test]
    fn test_meta_cache_path_top_level() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("myproject");
        create_project(&proj, &["src"]);
        let source = proj.join("src/main.wj");
        std::fs::write(&source, "").unwrap();

        let result = meta_cache_path(&source);
        assert_eq!(result, proj.join(".wj-cache/main.wj.meta"));
    }

    #[test]
    fn test_meta_cache_path_no_project_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("noproject");
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("file.wj");
        std::fs::write(&source, "").unwrap();

        let result = meta_cache_path(&source);
        assert_eq!(result, dir.join("file.wj.meta"));
    }

    #[test]
    fn test_meta_cache_path_components_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("uiproject");
        create_project(&proj, &["src/components"]);
        let source = proj.join("src/components/textarea.wj");
        std::fs::write(&source, "").unwrap();

        let result = meta_cache_path(&source);
        assert_eq!(
            result,
            proj.join(".wj-cache/components/textarea.wj.meta")
        );
    }

    #[test]
    fn test_meta_cache_path_components_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("uiproject");
        create_project(&proj, &["src/components/forms"]);
        let source = proj.join("src/components/forms/input.wj");
        std::fs::write(&source, "").unwrap();

        let result = meta_cache_path(&source);
        assert_eq!(
            result,
            proj.join(".wj-cache/components/forms/input.wj.meta")
        );
    }

    #[test]
    fn test_meta_cache_root_with_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("myproject");
        create_project(&proj, &["src"]);
        let src = proj.join("src");

        let result = meta_cache_root(&src);
        assert_eq!(result, proj.join(".wj-cache"));
    }

    #[test]
    fn test_meta_cache_root_no_project() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("noproject/src");
        std::fs::create_dir_all(&dir).unwrap();

        let result = meta_cache_root(&dir);
        assert_eq!(result, dir.join(".wj-cache"));
    }

    #[test]
    fn test_meta_cache_root_wj_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("wjproject");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("wj.toml"), "[project]\nname = \"test\"").unwrap();
        std::fs::create_dir_all(proj.join("src")).unwrap();
        let src = proj.join("src");

        let result = meta_cache_root(&src);
        assert_eq!(result, proj.join(".wj-cache"));
    }
}
