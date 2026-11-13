//! Path helpers for build output layout (`src/` subtree vs crate root).

use std::path::Path;

/// Module marker files: a directory holding one is a module directory, and a directory below it is a submodule.
/// (`lib.*` / `main.*` mark crate roots, which can sit beside an ordinary `build/` or `gen/` output, so they do not count.)
const MODULE_MARKERS: &[&str] = &["mod.wj", "mod.rs"];

/// How many directory levels above the output are searched for a module-tree marker.
const MODULE_TREE_SEARCH_DEPTH: usize = 8;

/// True when `output_dir` lies inside an existing module tree (e.g. `src/components/generated/`), so its `mod.rs` is a
/// submodule of a crate that is rooted elsewhere; false when it is a crate root of its own (e.g. `out/`, `gen/`).
///
/// Decided from the project's layout, never from what directories are called: the output is inside a module tree when a
/// strict ancestor holds a module marker file (`mod.wj`, `mod.rs`), or when it
/// lies inside the `src/` directory of the nearest project manifest (`wj.toml` / `Cargo.toml`, the manifest's default
/// source directory). The search stops at the project boundary.
pub(crate) fn is_submodule_output_dir(output_dir: &Path) -> bool {
    let abs = std::path::absolute(output_dir).unwrap_or_else(|_| output_dir.to_path_buf());
    let project = abs.parent().and_then(crate::metadata::find_project_root);
    if let Some(project) = &project {
        if abs.strip_prefix(project.join("src")).is_ok_and(|rel| !rel.as_os_str().is_empty()) {
            return true;
        }
    }
    for dir in abs.ancestors().skip(1).take(MODULE_TREE_SEARCH_DEPTH) {
        if MODULE_MARKERS.iter().any(|m| dir.join(m).is_file()) {
            return true;
        }
        if project.as_deref() == Some(dir) {
            break;
        }
    }
    false
}

/// True when `dir` is a known transpile output folder (`gen/`, `build/`, `generated/`).
/// Used to avoid treating stale generated `.rs` siblings as hand-written FFI modules.
pub(crate) fn is_transpile_output_directory(dir: &Path) -> bool {
    dir.file_name()
        .and_then(|n| n.to_str())
        .map(|name| matches!(name, "gen" | "build" | "generated"))
        .unwrap_or(false)
}

/// True when `dir` is a process/OS temp root (or similarly named ephemeral folder).
/// Sibling `*.rs` harvest must not mine `/tmp` (or macOS `…/T`) when `--output` lives there
/// (P3.561 / wj-migrate E0432 from leftover harness `*.rs`).
pub(crate) fn is_ephemeral_sibling_rs_source_dir(dir: &Path) -> bool {
    if let Ok(temp) = std::env::temp_dir().canonicalize() {
        if let Ok(canon) = dir.canonicalize() {
            if canon == temp {
                return true;
            }
        }
    }
    if dir == std::env::temp_dir() {
        return true;
    }
    matches!(
        dir.file_name().and_then(|n| n.to_str()),
        Some("tmp" | "temp" | ".tmp" | "Temp" | "TMP" | "Temporary Items")
    )
}

/// Directories that must never be mined for hand-written sibling `*.rs`.
pub(crate) fn must_not_harvest_sibling_rs(dir: &Path) -> bool {
    is_transpile_output_directory(dir) || is_ephemeral_sibling_rs_source_dir(dir)
}

pub(crate) fn source_dir_for_output(
    output_subdir: &Path,
    layout: Option<(&Path, &Path)>,
) -> Option<std::path::PathBuf> {
    let (out_root, src_root) = layout?;
    let rel = output_subdir.strip_prefix(out_root).ok()?;
    Some(src_root.join(rel))
}

/// True when `name.wj` or `name/mod.wj` exists anywhere under `src_root` (excluding the root itself).
pub(crate) fn wj_module_declared_in_subtree(src_root: &Path, name: &str) -> bool {
    fn walk(dir: &Path, name: &str, is_root: bool) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some(name)
                    && path.join("mod.wj").exists()
                {
                    return !is_root;
                }
                if walk(&path, name, false) {
                    return true;
                }
            } else if path.extension().and_then(|e| e.to_str()) == Some("wj")
                && path.file_stem().and_then(|s| s.to_str()) == Some(name)
                && !is_root
            {
                return true;
            }
        }
        false
    }
    walk(src_root, name, true)
}
