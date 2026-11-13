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
/// does not declare. `cache_root` is the shared cache root (the manifest-less metadata cache lives under it).
pub(super) fn merge_root(
    root: &Path,
    cache_root: &Path,
    registry: &mut crate::analyzer::SignatureRegistry,
    copy_structs: &mut Vec<String>,
    all_struct_fields: &mut HashMap<String, Vec<Vec<String>>>,
) {
    let prev = DECLARED.with(|c| c.replace(declared_by_crate_metadata(root)));
    let scoped = super::PEER_SCOPE.with(|c| c.borrow().is_none());
    if scoped {
        super::PEER_SCOPE.with(|c| *c.borrow_mut() = Some(PeerScope::new(root, None, cache_root)));
    }
    let meta_root = crate_metadata::meta_cache_root(root, cache_root);
    if meta_root.exists() {
        merge_wj_meta_signatures_from_dir_inner(&meta_root, Some(cache_root), registry, copy_structs, all_struct_fields);
    }
    merge_wj_meta_signatures_from_dir_inner(root, Some(cache_root), registry, copy_structs, all_struct_fields);
    if scoped {
        super::PEER_SCOPE.with(|c| *c.borrow_mut() = None);
    }
    DECLARED.with(|c| *c.borrow_mut() = prev);
}

/// The `.wj` source a `.wj.meta` was written for: `<project>/.wj-cache/foo/bar.wj.meta` is `<project>/src/foo/bar.wj`
/// (or `<project>/foo/bar.wj`); an entry of a manifest-less tree (`<cache root>/meta/<tree key>/foo/bar.wj.meta`) is
/// `foo/bar.wj` under the tree root of the build being fed (the key names content, never a location); a legacy
/// `bar.wj.meta` beside its source is `bar.wj`. `cache_root` is the shared cache root (`None`: no manifest-less entry
/// is recognised).
pub(super) fn sidecar_source(meta: &Path, cache_root: Option<&Path>) -> Option<PathBuf> {
    let base = super::PEER_SCOPE.with(|c| c.borrow().as_ref().map(|s| s.base.clone()));
    sidecar_source_in(meta, base.as_deref(), cache_root)
}

/// [`sidecar_source`] with the tree root of manifest-less entries given.
fn sidecar_source_in(meta: &Path, base: Option<&Path>, cache_root: Option<&Path>) -> Option<PathBuf> {
    let beside = meta.with_extension("");
    if beside.is_file() {
        return Some(beside);
    }
    let comps: Vec<_> = meta.components().collect();
    if let Some(at) = comps.iter().position(|c| c.as_os_str() == ".wj-cache") {
        let project: PathBuf = comps[..at].iter().collect();
        let rel: PathBuf = comps[at + 1..].iter().collect();
        let rel = rel.with_extension("");
        return [project.join("src").join(&rel), project.join(&rel)].into_iter().find(|p| p.is_file());
    }
    let tree_rel = meta.strip_prefix(crate_metadata::global_meta_cache_dir(cache_root?)).ok()?;
    let rel: PathBuf = tree_rel.components().skip(1).collect();
    Some(base?.join(rel.with_extension(""))).filter(|p| p.is_file())
}

/// Identifiers in one `use` path segment or declaration text (`shapes::{Circle, Square}` -> shapes, Circle, Square).
pub(super) fn identifiers(text: &str, out: &mut std::collections::BTreeSet<String>) {
    for word in text.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if !word.is_empty() && !word.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            out.insert(word.to_string());
        }
    }
}

/// The module a cached sidecar describes, as the build names it: its file stem (`geo/shapes.wj.meta` is `shapes`;
/// `geo/mod.wj.meta` is `geo`).
fn sidecar_module(meta: &Path) -> Option<String> {
    let name = meta.file_name()?.to_str()?.strip_suffix(".wj.meta")?;
    if name == "mod" {
        return meta.parent()?.file_name()?.to_str().map(str::to_string);
    }
    Some(name.to_string())
}

/// The peers a build may read: their sources live under `base`; `reached` (when known) holds the modules the build's
/// sources name, closed over the `use` declarations of the reached peers' own sources.
pub(super) struct PeerScope {
    pub(super) base: PathBuf,
    reached: Option<std::collections::BTreeSet<String>>,
}

impl PeerScope {
    pub(super) fn new(source_root: &Path, uses: Option<std::collections::BTreeSet<String>>, cache_root: &Path) -> PeerScope {
        let base = crate_metadata::peer_source_base(source_root);
        let mut scope = PeerScope { base, reached: None };
        if let Some(seed) = uses {
            scope.reached = Some(scope.close_over_peers(seed, &crate_metadata::meta_cache_root(source_root, cache_root), cache_root));
        }
        scope
    }

    /// `seed` plus the identifiers named by the `use` / `mod` declarations of every reached peer source, to a fixed point.
    fn close_over_peers(
        &self,
        mut reached: std::collections::BTreeSet<String>,
        meta_root: &Path,
        cache_root: &Path,
    ) -> std::collections::BTreeSet<String> {
        let mut metas = Vec::new();
        collect_metas(meta_root, 0, &mut metas);
        metas.sort();
        let mut done = vec![false; metas.len()];
        loop {
            let mut grew = false;
            for (i, meta) in metas.iter().enumerate() {
                if done[i] || !sidecar_module(meta).is_some_and(|m| reached.contains(&m)) {
                    continue;
                }
                done[i] = true;
                let Some(text) = sidecar_source_in(meta, Some(&self.base), Some(cache_root)).and_then(|src| std::fs::read_to_string(src).ok()) else {
                    continue;
                };
                for line in text.lines() {
                    let line = line.trim_start();
                    let decl = line.strip_prefix("pub ").unwrap_or(line);
                    if let Some(rest) = decl.strip_prefix("use ").or_else(|| decl.strip_prefix("mod ")) {
                        let before = reached.len();
                        identifiers(rest.split(';').next().unwrap_or(rest), &mut reached);
                        grew |= reached.len() > before;
                    }
                }
            }
            if !grew {
                return reached;
            }
        }
    }

    /// True when the sidecar at `meta` describes a module this build reaches (always, when reachability is unknown).
    pub(super) fn reaches(&self, meta: &Path) -> bool {
        match &self.reached {
            None => true,
            Some(r) => sidecar_module(meta).is_some_and(|m| r.contains(&m)),
        }
    }
}

fn collect_metas(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 12 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_metas(&p, depth + 1, out);
        } else if p.to_string_lossy().ends_with(".wj.meta") {
            out.push(p);
        }
    }
}
