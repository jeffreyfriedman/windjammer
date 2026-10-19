//! Which `.wj` files make up a program, and where each sits in the module tree.
//!
//! One walker for every consumer: the compiler's project build (`compiler::find_wj_files`), the
//! resolver (`resolve::module_path_from_file`) and the interpreter's project loader. Interpreted and
//! compiled runs therefore see the same files under the same module paths by construction.

use std::path::{Path, PathBuf};

/// Directory names never searched for sources (build output, VCS, vendored JS).
pub const SKIPPED_DIRS: [&str; 5] = ["build", "gen", "target", ".git", "node_modules"];

/// Every `.wj` file under `dir` (recursive, sorted), skipping [`SKIPPED_DIRS`].
pub fn wj_files_under(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walk(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wj") {
            files.push(path);
        } else if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !SKIPPED_DIRS.contains(&name) {
                walk(&path, files)?;
            }
        }
    }
    Ok(())
}

/// One source file of a program and its module path (`[]` = crate root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub module_path: Vec<String>,
    pub file: PathBuf,
}

/// The files of the program rooted at `entry` inside `source_root`, entry first.
///
/// Excluded (they are not modules of this binary): test sources (`*_test.wj`, `tests/`), and any
/// sub-directory that holds its own `main.wj` (another binary, e.g. `cmd/tool/main.wj`) or sits under
/// `bin/` (Cargo-style `src/bin/*.wj`). Module paths follow [`crate::resolve::module_path_from_file`].
pub fn program_sources(source_root: &Path, entry: &Path) -> std::io::Result<Vec<SourceFile>> {
    let entry_canon = canon(entry);
    let mut out = vec![SourceFile {
        module_path: Vec::new(),
        file: entry.to_path_buf(),
    }];
    for f in wj_files_under(source_root)? {
        if canon(&f) == entry_canon || is_excluded(source_root, &f) {
            continue;
        }
        let rel = f.strip_prefix(source_root).unwrap_or(&f);
        // the crate root is the entry; other root-level `main.wj`/`lib.wj` are not modules of it
        if rel.parent().is_none_or(|p| p.as_os_str().is_empty())
            && matches!(rel.to_str(), Some("main.wj") | Some("lib.wj"))
        {
            continue;
        }
        out.push(SourceFile {
            module_path: crate::resolve::module_path_from_file(source_root, &f),
            file: f,
        });
    }
    Ok(out)
}

fn canon(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

fn is_excluded(source_root: &Path, file: &Path) -> bool {
    if crate::test_module_gate::is_test_source_path(file.strip_prefix(source_root).unwrap_or(file)) {
        return true;
    }
    let rel = file.strip_prefix(source_root).unwrap_or(file);
    let mut dir = source_root.to_path_buf();
    let comps: Vec<_> = rel.components().collect();
    for (i, c) in comps.iter().enumerate() {
        if i + 1 == comps.len() {
            break;
        }
        dir.push(c);
        if c.as_os_str() == "bin" && i == 0 {
            return true;
        }
        if dir.join("main.wj").is_file() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(root: &Path, rel: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "fn main() {}\n").unwrap();
    }

    fn paths(v: &[SourceFile]) -> Vec<String> {
        v.iter().map(|s| s.module_path.join("::")).collect()
    }

    #[test]
    fn program_sources_maps_files_to_module_paths_and_skips_other_roots() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        for f in [
            "main.wj",
            "greet.wj",
            "util/mod.wj",
            "util/math.wj",
            "plain/a.wj",
            "greet_test.wj",
            "tests/t.wj",
            "bin/tool.wj",
            "cmd/other/main.wj",
            "cmd/other/helper.wj",
            "target/x.wj",
        ] {
            touch(r, f);
        }
        let got = program_sources(r, &r.join("main.wj")).unwrap();
        assert_eq!(got[0].module_path, Vec::<String>::new());
        let mut p = paths(&got[1..]);
        p.sort();
        assert_eq!(p, vec!["greet", "plain::a", "util", "util::math"]);
    }
}
