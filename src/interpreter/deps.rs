//! Windjammer dependencies loaded as source: the project's `wj.toml` graph (`package::graph`, the same
//! one the compiled build hands to Cargo), each library mounted as a top-level crate module.

use super::error::{ErrorKind, RuntimeError};
use super::project::{parse_files, LoadedFile};
use crate::module_system::sources::{program_sources, wj_files_under, SourceFile};
use crate::package::graph::{self, DepKind};
use std::path::Path;

/// Dependencies of the project that contains `entry`.
#[derive(Default)]
pub struct Dependencies {
    /// `(crate name, files)` for every Windjammer dependency, loaded as source.
    pub windjammer: Vec<(String, Vec<LoadedFile>)>,
    /// Crate names of Rust dependencies (bridged per ADR-0023, never interpreted).
    pub rust: Vec<String>,
}

pub fn dependency_sources(entry: &Path) -> Result<Dependencies, RuntimeError> {
    let Some(root) = crate::module_system::project::find_manifest_dir(entry) else {
        return Ok(Dependencies::default());
    };
    let mut out: Vec<(String, Vec<LoadedFile>)> = Vec::new();
    let mut rust: Vec<String> = Vec::new();
    // A dependency's own dependencies are mounted too (`wj_gitignore` calls `wj_glob::is_match`), each crate once.
    let mut pending = vec![root.clone()];
    let mut seen_roots: Vec<std::path::PathBuf> = vec![root];
    while let Some(project) = pending.pop() {
        let nodes = graph::resolve(&project).map_err(|e| RuntimeError::new(ErrorKind::Semantic, e))?;
        for n in nodes.iter().filter(|n| n.kind == DepKind::Rust) {
            if !rust.contains(&n.crate_name()) {
                rust.push(n.crate_name());
            }
        }
        for n in nodes.into_iter().filter(|n| n.kind == DepKind::Windjammer) {
            let Some(src) = n.source_root() else { continue };
            if out.iter().any(|(name, _)| *name == n.crate_name()) {
                continue;
            }
            if let Some(dep_root) = n.root.as_ref().filter(|r| !seen_roots.contains(r)) {
                seen_roots.push(dep_root.clone());
                pending.push(dep_root.clone());
            }
            let files: Vec<SourceFile> = match n.lib_entry() {
                Some(lib) => program_sources(&src, &lib),
                None => wj_files_under(&src).map(|fs| {
                    fs.into_iter()
                        .filter(|f| !crate::test_module_gate::is_test_source_path(f.strip_prefix(&src).unwrap_or(f)))
                        .filter(|f| f.file_name().is_none_or(|n| n != "main.wj"))
                        .map(|f| SourceFile { module_path: crate::resolve::module_path_from_file(&src, &f), file: f })
                        .collect()
                }),
            }
            .map_err(|e| RuntimeError::new(ErrorKind::Semantic, format!("dependency `{}`: {}", n.name, e)))?;
            out.push((n.crate_name(), parse_files(&files)?));
        }
    }
    Ok(Dependencies { windjammer: out, rust })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn dependencies_of_dependencies_are_mounted_once() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        write(r, "app/wj.toml", "[package]\nname = \"app\"\n\n[dependencies]\nwj_a = { path = \"../a/build\" }\nwj_b = { path = \"../b\" }\n");
        write(r, "app/src/main.wj", "fn main() {}\n");
        write(r, "a/wj.toml", "[package]\nname = \"a\"\n\n[lib]\n\n[dependencies]\nwj_b = { path = \"../b/build\" }\n");
        write(r, "a/src/lib.wj", "pub fn f() -> int { wj_b::g() }\n");
        write(r, "a/build/Cargo.toml", "[package]\nname = \"a\"\n");
        write(r, "b/wj.toml", "[package]\nname = \"b\"\n\n[lib]\n");
        write(r, "b/src/lib.wj", "pub fn g() -> int { 1 }\n");
        let deps = dependency_sources(&r.join("app/src/main.wj")).unwrap();
        let mut names: Vec<&str> = deps.windjammer.iter().map(|(n, _)| n.as_str()).collect();
        names.sort();
        assert_eq!(names, ["wj_a", "wj_b"]);
        assert!(deps.rust.is_empty());
    }
}
