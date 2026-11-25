//! Discovery of the `.wj` files a `check` run covers.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// All `.wj` files under `path` (a file or a directory), sorted for deterministic output.
pub fn collect(path: &Path) -> Result<Vec<PathBuf>> {
    Ok(collect_scoped(path, false)?.0)
}

/// As `collect`, with what a directory walk left out (generated and ignored directories, `scope`); `no_ignore`
/// reads everything.
pub fn collect_scoped(path: &Path, no_ignore: bool) -> Result<(Vec<PathBuf>, super::scope::Scope)> {
    let none = super::scope::Scope::default;
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }
    // A lone script (no `wj.toml` above it) is checked as the program it starts: the file and the sibling
    // modules it names with `mod`/`use`, as the interpreter and `wj build` load them; unrelated `.wj` files
    // next to it are not part of it.
    if path.is_file() && crate::module_system::project::find_manifest_dir(path).is_none() {
        return Ok((reachable(path), none()));
    }
    // A crate root (`main.wj` / `lib.wj`) is checked as the whole program it starts: its bodiless `mod x`
    // declarations load sibling files, exactly as the interpreter and the compiler see them.
    if path.is_file() && path.file_name().is_some_and(|n| n == "main.wj" || n == "lib.wj") {
        use crate::module_system::{project, sources};
        let root = project::source_root_for(path, project::find_manifest_dir(path).as_deref());
        if let Ok(files) = sources::program_sources(&root, path) {
            let mut out: Vec<PathBuf> = files.into_iter().map(|f| f.file).collect();
            out.sort();
            out.dedup();
            return Ok((out, none()));
        }
    }
    if path.is_file() {
        return Ok((if path.extension().is_some_and(|e| e == "wj") { vec![path.to_path_buf()] } else { Vec::new() }, none()));
    }
    super::scope::walk(path, no_ignore)
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

/// `entry` and the files reachable from it through `mod name` and `use a::b::..` (each segment tried as
/// `a.wj` and `a/mod.wj` below the entry's directory, then below each found module's own directory).
pub(crate) fn reachable(entry: &Path) -> Vec<PathBuf> {
    let root = entry.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut out: Vec<PathBuf> = vec![entry.to_path_buf()];
    let mut i = 0;
    while i < out.len() {
        let file = out[i].clone();
        i += 1;
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        // a module's own children live next to it (`a/mod.wj` -> `a/`, `a.wj` -> `a/`)
        let here = if file.file_name().is_some_and(|n| n == "mod.wj") || file == entry {
            file.parent().unwrap_or(&root).to_path_buf()
        } else {
            file.with_extension("")
        };
        for segs in module_paths(&text) {
            for base in [&root, &here] {
                let mut dir = base.clone();
                for s in &segs {
                    for cand in [dir.join(format!("{s}.wj")), dir.join(s).join("mod.wj")] {
                        if cand.is_file() && !out.contains(&cand) {
                            out.push(cand);
                        }
                    }
                    dir = dir.join(s);
                }
            }
        }
    }
    out.sort();
    out
}

/// The module paths a source names: `mod a` and `use a::b::..` (`std`, `crate`, `self`, `super` heads and the
/// last segment of a `use`, which names an item, are left out of the search).
fn module_paths(text: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let l = line.trim().trim_start_matches("pub ").trim();
        if let Some(rest) = l.strip_prefix("mod ") {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                out.push(vec![name]);
            }
        } else if let Some(rest) = l.strip_prefix("use ") {
            let path: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':').collect();
            let segs: Vec<String> = path.split("::").filter(|s| !s.is_empty()).map(String::from).collect();
            if segs.first().is_some_and(|h| !matches!(h.as_str(), "std" | "crate" | "self" | "super")) {
                out.push(segs);
            }
        }
    }
    out
}
