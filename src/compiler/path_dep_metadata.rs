//! Signatures of a `wj.toml` path dependency that ships no `metadata.json`: derived from the dependency's own
//! Windjammer sources, never guessed.
//!
//! A consumer reads a dependency's formals (`width(self, w: &str)`) from its `metadata.json`. When the dependency
//! has Windjammer sources but no metadata yet, those sources are analyzed as a library into the Windjammer cache
//! (`<cache root>/path-deps/<crate>-<fingerprint>/`), keyed by the compiler identity, the dependency's location
//! and its sources, and the metadata written there is used. (A project's own `.wj-cache` is cleared when the
//! compiler changes, after dependencies are resolved, so it cannot hold them.) When that analysis fails, the build reports
//! which dependency has no signatures instead of assuming owned formals.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// The Windjammer source directory of a dependency root: `<dep>/src` when it holds `.wj` files, else `<dep>`.
pub(crate) fn wj_source_dir(dep_root: &Path) -> Option<PathBuf> {
    [dep_root.join("src"), dep_root.to_path_buf()]
        .into_iter()
        .find(|dir| super::dependency_resolution::find_wj_files(dir).is_ok_and(|files| !files.is_empty()))
}

fn fingerprint(source_dir: &Path, canonical: &Path, files: &[PathBuf]) -> String {
    let mut hasher = DefaultHasher::new();
    super::incremental::compiler_build_identity().hash(&mut hasher);
    canonical.hash(&mut hasher);
    let mut sorted: Vec<&PathBuf> = files.iter().collect();
    sorted.sort();
    for f in sorted {
        f.strip_prefix(source_dir).unwrap_or(f).to_string_lossy().hash(&mut hasher);
        std::fs::read_to_string(f).unwrap_or_default().hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// Metadata root for the path dependency `key` at `dep_root` (no `metadata.json` of its own), derived from its
/// Windjammer sources. `None` when the dependency has no Windjammer sources (a Rust crate); an `Err` names the
/// dependency when its sources could not be analyzed.
pub(crate) fn derive(key: &str, dep_root: &Path) -> Option<Result<PathBuf, String>> {
    let source_dir = wj_source_dir(dep_root)?;
    let files = super::dependency_resolution::find_wj_files(&source_dir).ok()?;
    let canonical = std::fs::canonicalize(&source_dir).unwrap_or_else(|_| source_dir.clone());
    let out = crate::cargo_cache::cache_root().join("path-deps").join(format!("{key}-{}", fingerprint(&source_dir, &canonical, &files)));
    // A dependency whose derivation is under way further up the chain: a cycle of path dependencies.
    let marker = out.join(".wj-deriving");
    if marker.is_file() {
        return Some(Err(format!("path dependency `{key}` ({}) depends on itself", dep_root.display())));
    }
    if out.join("metadata.json").is_file() {
        let index = crate::cargo_cache::path_dep_crate_index(&source_dir);
        if std::fs::read_to_string(&index).ok().as_deref() != Some(&out.display().to_string()) {
            if let Some(parent) = index.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&index, out.display().to_string());
        }
        return Some(Ok(out));
    }
    let _ = std::fs::create_dir_all(&out).and_then(|()| std::fs::write(&marker, ""));
    let built = super::compilation_pipeline::build_project_with_config(
        &source_dir,
        &out,
        crate::CompilationTarget::Rust,
        false,
        true,
        &[],
        &crate::compiler_config::CompilerConfig::default(),
    );
    let _ = std::fs::remove_file(&marker);
    if built.is_ok() && out.join("Cargo.toml").is_file() {
        let index = crate::cargo_cache::path_dep_crate_index(&source_dir);
        if let Some(parent) = index.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&index, out.display().to_string());
    }
    Some(match built {
        Ok(()) if out.join("metadata.json").is_file() => Ok(out),
        Ok(()) => Err(format!("path dependency `{key}` ({}): its sources produced no metadata.json", dep_root.display())),
        Err(e) => Err(format!("path dependency `{key}` ({}): its sources could not be analyzed: {e}", dep_root.display())),
    })
}
