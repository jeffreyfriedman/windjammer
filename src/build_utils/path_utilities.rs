//! Path helpers for build output layout (`src/` subtree vs crate root).

use std::path::Path;

/// True when the output directory is a Rust module subtree under `src/...` (not the crate root).
/// In that case we only emit `mod.rs` for Cargo, not `lib.rs` (which would be invalid as a nested crate root).
pub(crate) fn is_submodule_output_dir(output_dir: &Path) -> bool {
    let comps: Vec<_> = output_dir.components().collect();
    for i in 0..comps.len() {
        if comps[i].as_os_str() == "src" && i + 1 < comps.len() {
            return true;
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
