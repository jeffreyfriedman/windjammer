//! Build cache helpers: incremental-safe writes and stale Cargo.toml cleanup.

use std::path::Path;

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
