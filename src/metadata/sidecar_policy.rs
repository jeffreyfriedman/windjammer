//! Which cached sidecars may feed a build, and how they rank against declared sources.
//!
//! A `.wj.meta` under a `.wj-cache` is an analysis cache. It never outranks what a crate declares: while a root is
//! merged, the functions and structs that the root's crate `metadata.json` declares are dropped from its sidecars, so a
//! sidecar can only fill what the metadata does not say (a sidecar describing the same signature through another path
//! once widened a `u32` formal to `i32`). Whether a sidecar is stale is decided from its source file's content hash
//! and the compiler build (`wj_meta_is_current` in `mod.rs`).

use super::{crate_metadata, merge_wj_meta_signatures_from_dir_inner, ModuleMetadata, DECLARED};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Function and struct names a crate's own `metadata.json` declares.
#[derive(Default, Clone)]
pub(super) struct Declared {
    functions: HashSet<String>,
    structs: HashSet<String>,
}

/// The crate metadata `root` is described by: `root` itself or its project's `metadata.json`.
fn declared_by_crate_metadata(root: &Path) -> Option<Declared> {
    let path = crate_metadata::resolve_metadata_json_path(root).or_else(|| {
        let p = crate_metadata::find_project_root(root)?.join("metadata.json");
        p.is_file().then_some(p)
    })?;
    let meta: crate_metadata::CrateMetadata = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    Some(Declared { functions: meta.functions.into_keys().collect(), structs: meta.structs.into_keys().collect() })
}

/// Remove from `meta` what `declared` declares.
pub(super) fn drop_declared(declared: Option<&Declared>, meta: &mut ModuleMetadata) {
    if let Some(d) = declared {
        meta.functions.retain(|k, _| !d.functions.contains(k));
        meta.structs.retain(|k, _| !d.structs.contains(k));
    }
}

/// Merge the sidecars of `root` (its cache directory and the tree itself), restricted to what `root`'s crate metadata
/// does not declare.
pub(super) fn merge_root(
    root: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    copy_structs: &mut Vec<String>,
    all_struct_fields: &mut HashMap<String, Vec<Vec<String>>>,
) {
    let prev = DECLARED.with(|c| c.replace(declared_by_crate_metadata(root)));
    let cache_root = crate_metadata::meta_cache_root(root);
    if cache_root.exists() {
        merge_wj_meta_signatures_from_dir_inner(&cache_root, registry, copy_structs, all_struct_fields);
    }
    merge_wj_meta_signatures_from_dir_inner(root, registry, copy_structs, all_struct_fields);
    DECLARED.with(|c| *c.borrow_mut() = prev);
}

/// The `.wj` source a `.wj.meta` was written for: `<project>/.wj-cache/foo/bar.wj.meta` is `<project>/src/foo/bar.wj`
/// (or `<project>/foo/bar.wj`); a legacy `bar.wj.meta` beside its source is `bar.wj`.
fn sidecar_source(meta: &Path) -> Option<PathBuf> {
    let beside = meta.with_extension("");
    if beside.is_file() {
        return Some(beside);
    }
    let comps: Vec<_> = meta.components().collect();
    let at = comps.iter().position(|c| c.as_os_str() == ".wj-cache")?;
    let project: PathBuf = comps[..at].iter().collect();
    let rel: PathBuf = comps[at + 1..].iter().collect();
    let rel = rel.with_extension("");
    [project.join("src").join(&rel), project.join(&rel)].into_iter().find(|p| p.is_file())
}

