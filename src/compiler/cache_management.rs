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
/// When the compiler binary itself has changed, ALL files are marked dirty.
/// Returns (dirty_files, skipped_count).
pub fn compute_dirty_files(
    wj_files: &[(PathBuf, String)],
    src_base: &Path,
    output: &Path,
) -> (Vec<usize>, usize) {
    let compiler_changed = !is_compiler_stamp_fresh(output);
    if compiler_changed {
        return ((0..wj_files.len()).collect(), 0);
    }

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

/// Stamp file name written into the output directory to track which compiler
/// version produced the current generated files.  When the compiler binary
/// changes (rebuild, upgrade, etc.) this stamp becomes stale and the next
/// `wj build` automatically re-transpiles everything — no manual sync needed.
const COMPILER_STAMP_FILE: &str = ".wj-compiler-stamp";

/// Return the mtime of the currently running compiler binary, if available.
fn compiler_binary_mtime() -> Option<SystemTime> {
    std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
}

/// Check whether the compiler stamp in `output` matches the current binary.
/// Returns `false` (= stale) when:
///   - the stamp file doesn't exist yet,
///   - the stamp is older than the running compiler binary, or
///   - the compiler version recorded in the stamp differs.
pub fn is_compiler_stamp_fresh(output: &Path) -> bool {
    let stamp_path = output.join(COMPILER_STAMP_FILE);
    let stamp_mtime = match std::fs::metadata(&stamp_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };

    if let Some(compiler_mtime) = compiler_binary_mtime() {
        if compiler_mtime > stamp_mtime {
            return false;
        }
    }

    match std::fs::read_to_string(&stamp_path) {
        Ok(content) => content.trim() == env!("CARGO_PKG_VERSION"),
        Err(_) => false,
    }
}

/// Write (or refresh) the compiler stamp in the output directory.
/// Called after a successful transpilation so subsequent builds can detect
/// when the compiler itself has been upgraded.
pub fn write_compiler_stamp(output: &Path) -> std::io::Result<()> {
    let stamp_path = output.join(COMPILER_STAMP_FILE);
    std::fs::write(&stamp_path, format!("{}\n", env!("CARGO_PKG_VERSION")))
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
