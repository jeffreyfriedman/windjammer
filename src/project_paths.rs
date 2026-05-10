//! Source-root detection and output paths (shared by `build_project` and `compiler`).

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Find the actual source root for a Windjammer file.
///
/// Walks up from the file looking for a directory that is a project source root.
/// Priority order (first match closest to the file wins):
///   1. `src` — only if it looks like a real project source dir
///              (sibling Cargo.toml, or contains mod.wj / .wj files)
///   2. Topmost directory containing mod.wj
///   3. The file's immediate parent (standalone file fallback)
pub fn find_source_root(file_path: &Path) -> Option<&Path> {
    let mut current = file_path;
    let mut topmost_mod_wj_dir = None;
    let mut found_project_src: Option<&Path> = None;

    while let Some(parent) = current.parent() {
        if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
            if dir_name == "src" && found_project_src.is_none() {
                if is_project_source_dir(parent) {
                    found_project_src = Some(parent);
                }
            }
        }

        if parent.join("mod.wj").exists() {
            topmost_mod_wj_dir = Some(parent);
        }

        current = parent;
    }

    if let Some(src) = found_project_src {
        return Some(src);
    }

    if let Some(mod_wj_dir) = topmost_mod_wj_dir {
        return Some(mod_wj_dir);
    }

    file_path.parent()
}

/// Check if a `src/` directory is a real project source root, not just any
/// directory named "src" (e.g. `/Users/dev/src/` is a personal code directory).
fn is_project_source_dir(src_dir: &Path) -> bool {
    if let Some(project_dir) = src_dir.parent() {
        if project_dir.join("Cargo.toml").exists() {
            return true;
        }
        if project_dir.join("mod.wj").exists() {
            return true;
        }
    }
    if src_dir.join("mod.wj").exists() {
        return true;
    }
    if src_dir.join("lib.wj").exists() || src_dir.join("main.wj").exists() {
        return true;
    }
    
    // TDD FIX: Recognize bare src/ directories with .wj files (even without mod.wj)
    // This fixes the case where `wj build src/ecs/entity.wj` should use `src/` as root.
    if src_dir.file_name().and_then(|n| n.to_str()) == Some("src") {
        // If the src/ directory contains any .wj files (directly or in subdirs), it's a source root
        if let Ok(entries) = fs::read_dir(src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("wj") {
                    return true;
                }
                if path.is_dir() {
                    // Check one level deep for .wj files (covers src/ecs/entity.wj case)
                    if let Ok(sub_entries) = fs::read_dir(&path) {
                        if sub_entries
                            .flatten()
                            .any(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("wj"))
                        {
                            return true;
                        }
                    }
                }
            }
        }
    }
    
    false
}

/// Calculate output path that preserves directory structure
///
/// Example:
/// - source_root: "windjammer-game/src"
/// - input_path: "windjammer-game/src/math/vec2.wj"
/// - output_dir: "build"
/// - Result: "build/math/vec2.rs"
pub fn get_relative_output_path(
    source_root: &Path,
    input_path: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    // Get the relative path from source_root to input_path
    let relative = input_path.strip_prefix(source_root).unwrap_or(input_path);

    // Get the base name without extension
    let base_name = relative
        .file_stem()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    // TDD FIX: Check if there's a directory module with the same name
    // If both window.wj and window/ exist, put window.wj content into window/mod.rs
    // This prevents E0761: file for module `window` found at both "window.rs" and "window/mod.rs"
    let source_dir_for_module = if let Some(parent) = input_path.parent() {
        parent.join(base_name)
    } else {
        PathBuf::from(base_name)
    };

    let has_directory_module = source_dir_for_module.is_dir()
        && source_dir_for_module
            .read_dir()
            .map(|mut entries| {
                entries.any(|e| {
                    e.ok()
                        .and_then(|e| e.file_name().into_string().ok())
                        .map(|name| name.ends_with(".wj"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

    // Replace .wj extension with .rs
    let rs_filename = relative
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.replace(".wj", ".rs"))
        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;

    // Construct output path preserving directory structure
    let mut output_path = output_dir.to_path_buf();

    // Add parent directories if they exist
    if let Some(parent) = relative.parent() {
        if parent != Path::new("") {
            output_path.push(parent);
        }
    }

    // mod.wj code (traits, structs, impls) goes to _mod_items.rs so it doesn't
    // conflict with the mod.rs generated by the module system. The module system
    // will append _mod_items.rs content into mod.rs after generating declarations.
    if base_name == "mod" {
        output_path.push("_mod_items.rs");
    } else if has_directory_module {
        output_path.push(base_name);
        output_path.push("mod.rs");
    } else {
        // Add the .rs filename
        output_path.push(rs_filename);
    }

    Ok(output_path)
}

/// Output path for a `.wj` file (directory-module layout when `stem/stem.wj` + `stem/*.wj` co-exist).
/// Deletes a stale flat `stem.rs` when emitting `stem/mod.rs` so rustc never sees both (E0761).
///
/// When `library` is true and the source is `mod.wj`, the compiled content goes to
/// `_mod_items.rs` instead of `mod.rs`. This prevents the `--module-file` pass from
/// overwriting code defined in `mod.wj` (structs, traits, impls). The module-file
/// generator in `build_utils.rs` merges `_mod_items.rs` back into `mod.rs`.
pub fn resolve_wj_output_path(
    source_root: &Path,
    wj_file: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    resolve_wj_output_path_ext(source_root, wj_file, output_dir, false)
}

pub fn resolve_wj_output_path_library(
    source_root: &Path,
    wj_file: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    resolve_wj_output_path_ext(source_root, wj_file, output_dir, true)
}

fn resolve_wj_output_path_ext(
    source_root: &Path,
    wj_file: &Path,
    output_dir: &Path,
    library: bool,
) -> Result<PathBuf> {
    let path = get_relative_output_path(source_root, wj_file, output_dir)?;
    if path.file_name().and_then(|s| s.to_str()) == Some("mod.rs") {
        if let Ok(rel) = wj_file.strip_prefix(source_root) {
            let stale = output_dir.join(rel.with_extension("rs"));
            if stale != path {
                let _ = fs::remove_file(stale);
            }
        }
        // In library mode, redirect mod.wj output to _mod_items.rs so that
        // the --module-file pass can merge it without overwriting.
        if library {
            let items_path = path.with_file_name("_mod_items.rs");
            return Ok(items_path);
        }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_relative_output_path_nested() {
        let source_root = Path::new("src");
        let input_path = Path::new("src/math/vec2.wj");
        let output_dir = Path::new("build");

        let result = get_relative_output_path(source_root, input_path, output_dir).unwrap();
        assert_eq!(result, PathBuf::from("build/math/vec2.rs"));
    }

    #[test]
    fn test_get_relative_output_path_flat() {
        let source_root = Path::new("src");
        let input_path = Path::new("src/vec2.wj");
        let output_dir = Path::new("build");

        let result = get_relative_output_path(source_root, input_path, output_dir).unwrap();
        assert_eq!(result, PathBuf::from("build/vec2.rs"));
    }

    #[test]
    fn test_get_relative_output_path_deeply_nested() {
        let source_root = Path::new("game/src");
        let input_path = Path::new("game/src/rendering/shaders/vertex.wj");
        let output_dir = Path::new("build");

        let result = get_relative_output_path(source_root, input_path, output_dir).unwrap();
        assert_eq!(result, PathBuf::from("build/rendering/shaders/vertex.rs"));
    }
}
