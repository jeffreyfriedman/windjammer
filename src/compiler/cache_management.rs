//! Build cache helpers: incremental-safe writes, stale Cargo.toml cleanup,
//! compiler-version tracking, and source-level incremental compilation support.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Write file only if content has changed, preserving mtime for Cargo's
/// incremental compilation when the generated Rust is identical.
pub fn write_if_changed(path: &Path, content: &str) -> std::io::Result<bool> {
    if path.exists() {
        if let Ok(existing) = std::fs::read_to_string(path) {
            if existing == content {
                return Ok(false);
            }
        }
    }
    std::fs::write(path, content)?;
    Ok(true)
}

/// Check if a generated .rs file is still fresh relative to its .wj source.
/// Returns true if the output exists and is newer than (or same age as) the source.
///
/// NOTE: Mtime alone is NOT sufficient for codegen skip decisions — use
/// [`is_codegen_cache_valid`] which also validates source content fingerprints.
pub fn is_output_fresh(source: &Path, output: &Path) -> bool {
    let source_mtime = match std::fs::metadata(source).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let output_mtime = match std::fs::metadata(output).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    output_mtime >= source_mtime
}

/// Check if the meta cache file is also fresh relative to the source.
pub fn is_meta_fresh(source: &Path) -> bool {
    let meta_path = crate::metadata::meta_cache_path(source);
    is_output_fresh(source, &meta_path)
}

/// Authoritative check for whether generated Rust can be skipped.
///
/// Requires ALL of:
/// - Output `.rs` exists
/// - Output mtime >= source mtime (sanity)
/// - `.wj.meta` fingerprint matches current source content + compiler identity
///
/// This prevents silent stale-output bugs where `.rs` mtime is newer than `.wj`
/// but content is outdated (e.g. manual touch, wrong copy, partial build).
pub fn is_codegen_cache_valid(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    dep_roots: &[PathBuf],
) -> bool {
    crate::compiler::incremental::is_codegen_cache_valid(source, source_path, output_path, dep_roots)
}

fn is_mod_wj_source(source_path: &Path) -> bool {
    source_path.file_name().and_then(|n| n.to_str()) == Some("mod.wj")
}

fn is_mod_items_output(output_path: &Path) -> bool {
    output_path.file_name().and_then(|n| n.to_str()) == Some("_mod_items.rs")
}

/// Path to the module-system `mod.rs` that absorbs merged `mod.wj` output.
fn merged_mod_rs_for_mod_wj(
    source_path: &Path,
    src_base: &Path,
    output_dir: &Path,
) -> Option<PathBuf> {
    let relative = source_path.strip_prefix(src_base).ok()?;
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let mut mod_rs = output_dir.to_path_buf();
    if parent.as_os_str().len() > 0 {
        mod_rs.push(parent);
    }
    mod_rs.push("mod.rs");
    Some(mod_rs)
}

/// Library `mod.wj` codegen writes `_mod_items.rs`, then `generate_mod_file` merges it
/// into `mod.rs` and deletes `_mod_items.rs`. Stale detection must accept that path.
pub fn is_library_codegen_cache_valid(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    src_base: &Path,
    output_dir: &Path,
    dep_roots: &[PathBuf],
) -> bool {
    if is_mod_wj_source(source_path) && is_mod_items_output(output_path) && !output_path.exists() {
        if !crate::compiler::incremental::fingerprint_matches_cached(
            source,
            source_path,
            dep_roots,
        ) {
            return false;
        }
        if let Some(mod_rs) = merged_mod_rs_for_mod_wj(source_path, src_base, output_dir) {
            return mod_rs.exists() && is_output_fresh(source_path, &mod_rs);
        }
        return false;
    }
    is_codegen_cache_valid(source, source_path, output_path, dep_roots)
}

/// Compute the set of .wj files that need recompilation.
/// A file is "dirty" if its output is missing or fails [`is_codegen_cache_valid`].
/// When the compiler binary itself has changed, ALL files are marked dirty.
/// Returns (dirty_files, skipped_count).
pub fn compute_dirty_files(
    wj_files: &[(PathBuf, String)],
    src_base: &Path,
    output: &Path,
    dep_roots: &[PathBuf],
) -> (Vec<usize>, usize) {
    let compiler_changed = !is_compiler_stamp_fresh(output);
    if compiler_changed {
        return ((0..wj_files.len()).collect(), 0);
    }

    let mut dirty_indices = Vec::new();
    let mut skipped = 0;

    for (i, (file, source)) in wj_files.iter().enumerate() {
        let output_file = match crate::project_paths::resolve_wj_output_path_library(
            src_base, file, output,
        ) {
            Ok(p) => p,
            Err(_) => {
                dirty_indices.push(i);
                continue;
            }
        };

        if is_library_codegen_cache_valid(source, file, &output_file, src_base, output, dep_roots) {
            skipped += 1;
        } else {
            dirty_indices.push(i);
        }
    }

    (dirty_indices, skipped)
}

/// Like [`compute_dirty_files`] but also includes transitive importers (analysis rebuild set).
pub fn compute_rebuild_set(
    wj_files: &[(PathBuf, String)],
    src_base: &Path,
    output: &Path,
    dep_roots: &[PathBuf],
    dependency_graph: &crate::compiler::incremental::DependencyGraph,
) -> std::collections::HashSet<usize> {
    let (dirty_indices, _) = compute_dirty_files(wj_files, src_base, output, dep_roots);
    let dirty: std::collections::HashSet<usize> = dirty_indices.into_iter().collect();
    dependency_graph.transitive_dependents(&dirty)
}

pub fn is_compiler_stamp_fresh(output: &Path) -> bool {
    crate::compiler::incremental::is_compiler_stamp_fresh(output)
}

/// Write (or refresh) the compiler stamp in the output directory.
pub fn write_compiler_stamp(output: &Path) -> std::io::Result<()> {
    crate::compiler::incremental::write_compiler_stamp(output)
}

/// Check if ALL source files are fresh (whole-crate fast path).
/// Also checks dependency metadata freshness relative to the newest source mtime
/// and whether the compiler binary itself has changed since the last build.
pub fn all_sources_fresh(
    wj_files: &[(PathBuf, String)],
    src_base: &Path,
    output: &Path,
    dep_metadata_paths: &[PathBuf],
) -> bool {
    if !is_compiler_stamp_fresh(output) {
        return false;
    }

    let mut max_dep_mtime = SystemTime::UNIX_EPOCH;
    for dep_path in dep_metadata_paths {
        if let Ok(meta) = std::fs::metadata(dep_path) {
            if let Ok(mtime) = meta.modified() {
                if mtime > max_dep_mtime {
                    max_dep_mtime = mtime;
                }
            }
        }
    }

    for (file, source) in wj_files {
        let output_file = match crate::project_paths::resolve_wj_output_path_library(
            src_base, file, output,
        ) {
            Ok(p) => p,
            Err(_) => return false,
        };

        if !is_library_codegen_cache_valid(
            source,
            file,
            &output_file,
            src_base,
            output,
            dep_metadata_paths,
        ) {
            return false;
        }

        // If any dep metadata is newer than the output, the output is stale
        // (cross-crate ownership may have changed)
        if let Ok(out_meta) = std::fs::metadata(&output_file) {
            if let Ok(out_mtime) = out_meta.modified() {
                if max_dep_mtime > out_mtime {
                    return false;
                }
            }
        }
    }

    true
}

/// True when `source_path` belongs to the library crate under transpilation (not an
/// injected stdlib or other out-of-tree analysis source).
pub fn is_library_source_under_root(source_path: &Path, src_base: &Path) -> bool {
    source_path.strip_prefix(src_base).is_ok()
}

/// After a library build (or whole-crate skip), verify every `.wj` file has a
/// codegen output that passes [`is_codegen_cache_valid`].
///
/// Fails loudly instead of leaving stale `.rs` files that compile but run wrong code.
///
/// Out-of-tree sources (e.g. compiler stdlib injects for `use std::collections`) are
/// analysis-only and must not be validated against the user crate output directory.
pub fn find_stale_codegen_outputs(
    wj_files: &[(PathBuf, String)],
    src_base: &Path,
    output: &Path,
    dep_roots: &[PathBuf],
) -> Vec<PathBuf> {
    let mut stale = Vec::new();
    for (file, source) in wj_files {
        if !is_library_source_under_root(file, src_base) {
            continue;
        }
        let output_file = match crate::project_paths::resolve_wj_output_path_library(
            src_base, file, output,
        ) {
            Ok(p) => p,
            Err(_) => {
                stale.push(file.clone());
                continue;
            }
        };
        if !is_library_codegen_cache_valid(
            source,
            file,
            &output_file,
            src_base,
            output,
            dep_roots,
        ) {
            stale.push(file.clone());
        }
    }
    stale
}

/// Remove any Cargo.toml files nested under the output root.
/// Older compiler versions generated per-directory manifests; these confuse Cargo
/// into treating subdirectories as separate packages, causing cyclic dependency errors.
pub(crate) fn clean_nested_cargo_toml(output_dir: &Path) {
    fn visit(dir: &Path, root: &Path) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some("target") {
                    continue;
                }
                visit(&path, root);
            } else if path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml")
                && path.parent() != Some(root)
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    visit(output_dir, output_dir);
}
