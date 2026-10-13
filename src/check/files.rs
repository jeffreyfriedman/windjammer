//! Discovery of the `.wj` files a `check` run covers.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

const SKIP_DIRS: [&str; 6] = ["target", "build", ".git", "node_modules", "out", ".wj-cache"];

/// All `.wj` files under `path` (a file or a directory), sorted for deterministic output.
pub fn collect(path: &Path) -> Result<Vec<PathBuf>> {
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }
    let mut out = Vec::new();
    walk(path, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path.extension().is_some_and(|e| e == "wj") {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in std::fs::read_dir(path)? {
        let p = entry?.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if SKIP_DIRS.contains(&name) || name.starts_with('.') {
                continue;
            }
            walk(&p, out)?;
        } else if p.extension().is_some_and(|e| e == "wj") {
            out.push(p);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_build_dirs_and_sorts() {
        let dir = std::env::temp_dir().join(format!("wj_check_files_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("target/x.wj"), "").unwrap();
        std::fs::write(dir.join("src/b.wj"), "").unwrap();
        std::fs::write(dir.join("src/a.wj"), "").unwrap();
        std::fs::write(dir.join("src/notes.txt"), "").unwrap();
        let files = collect(&dir).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names, ["a.wj", "b.wj"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
