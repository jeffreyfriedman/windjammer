//! Build cache helpers: incremental-safe writes, stale Cargo.toml cleanup,
//! and source-level incremental compilation support.

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

/// Compute the set of .wj files that need recompilation.
/// A file is "dirty" if its .rs output doesn't exist or is older than the source.
/// Returns (dirty_files, skipped_count).
pub fn compute_dirty_files(
    wj_files: &[(PathBuf, String)],
    src_base: &Path,
    output: &Path,
) -> (Vec<usize>, usize) {
    let mut dirty_indices = Vec::new();
    let mut skipped = 0;

    for (i, (file, _)) in wj_files.iter().enumerate() {
        let output_file = match crate::project_paths::resolve_wj_output_path_library(
            src_base, file, output,
        ) {
            Ok(p) => p,
            Err(_) => {
                dirty_indices.push(i);
                continue;
            }
        };

        if is_output_fresh(file, &output_file) && is_meta_fresh(file) {
            skipped += 1;
        } else {
            dirty_indices.push(i);
        }
    }

    (dirty_indices, skipped)
}

/// Check if ALL source files are fresh (whole-crate fast path).
/// Also checks dependency metadata freshness relative to the newest source mtime.
pub fn all_sources_fresh(
    wj_files: &[(PathBuf, String)],
    src_base: &Path,
    output: &Path,
    dep_metadata_paths: &[PathBuf],
) -> bool {
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

    for (file, _) in wj_files {
        let output_file = match crate::project_paths::resolve_wj_output_path_library(
            src_base, file, output,
        ) {
            Ok(p) => p,
            Err(_) => return false,
        };

        if !is_output_fresh(file, &output_file) {
            return false;
        }
        if !is_meta_fresh(file) {
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
