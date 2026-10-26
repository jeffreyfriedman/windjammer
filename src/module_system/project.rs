//! `go run`-style project discovery for `wj run` / `wj test`, shared by the interpreted and compiled paths.
//!
//! - no path: find `wj.toml` upward from the working directory, run its default binary;
//! - a directory: its `main.wj`, `src/main.wj`, or the project's default binary;
//! - a file: that file;
//! - `--bin NAME`: a `[[bin]]` entry of `wj.toml`, `src/bin/NAME.wj`, or `cmd/NAME/main.wj`.
//!
//! The result names the entry file and the source root its modules live in; module layout inside the
//! source root is [`super::sources::program_sources`].

use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "wj.toml";

/// One runnable binary of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinTarget {
    pub name: String,
    pub entry: PathBuf,
}

/// What `wj run` executes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTarget {
    /// Directory holding `wj.toml` (None for a loose script or directory without a manifest).
    pub project_root: Option<PathBuf>,
    pub entry: PathBuf,
    /// Root of the entry's module tree (`src/`, the entry's own directory for `cmd/x/main.wj`, ...).
    pub source_root: PathBuf,
}

/// Nearest ancestor directory (including `start`) containing `wj.toml`.
pub fn find_manifest_dir(start: &Path) -> Option<PathBuf> {
    let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    let mut dir: &Path = if start.is_file() { start.parent()? } else { &start };
    loop {
        if dir.join(MANIFEST).is_file() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

fn package_name(root: &Path) -> Option<String> {
    let v: toml::Value = std::fs::read_to_string(root.join(MANIFEST)).ok()?.parse().ok()?;
    let name = v.get("package")?.get("name")?.as_str()?.to_string();
    (!name.is_empty()).then_some(name)
}

/// Binaries of the project at `root`, in priority order (the first is the default):
/// `[[bin]]` entries, `src/main.wj` (named after the package), `main.wj`, `cmd/*/main.wj`, `src/bin/*.wj`.
pub fn bin_targets(root: &Path) -> Vec<BinTarget> {
    let mut out: Vec<BinTarget> = Vec::new();
    let mut push = |name: String, entry: PathBuf| {
        if entry.is_file() && !out.iter().any(|b| b.name == name || b.entry == entry) {
            out.push(BinTarget { name, entry });
        }
    };
    let manifest: Option<toml::Value> =
        std::fs::read_to_string(root.join(MANIFEST)).ok().and_then(|s| s.parse().ok());
    if let Some(bins) = manifest.as_ref().and_then(|m| m.get("bin")).and_then(|b| b.as_array()) {
        for b in bins {
            if let (Some(n), Some(p)) = (b.get("name").and_then(|v| v.as_str()), b.get("path").and_then(|v| v.as_str())) {
                push(n.to_string(), root.join(p));
            }
        }
    }
    let pkg = package_name(root).unwrap_or_else(|| {
        root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "main".into())
    });
    push(pkg.clone(), root.join("src").join("main.wj"));
    push(pkg, root.join("main.wj"));
    for (dir, nested) in [(root.join("cmd"), true), (root.join("src").join("bin"), false)] {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if nested && p.is_dir() {
                push(name, p.join("main.wj"));
            } else if !nested && p.extension().is_some_and(|e| e == "wj") {
                push(name, p);
            }
        }
    }
    out
}

/// Source root for an entry file: its own directory for a binary that owns a directory
/// (`cmd/x/main.wj`, `src/bin/x/main.wj`) or a loose script, `src/` for `src/main.wj` and `src/bin/*.wj`.
pub fn source_root_for(entry: &Path, project_root: Option<&Path>) -> PathBuf {
    let parent = entry.parent().map(Path::to_path_buf).unwrap_or_default();
    if let Some(root) = project_root {
        let src = root.join("src");
        let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        let pc = canon(&parent);
        if pc == canon(&src.join("bin")) {
            return src;
        }
        if pc == canon(&src) || pc == canon(root) {
            return parent;
        }
        // any other directory (cmd/x/, tools/y/): the binary's own directory is its crate
        return parent;
    }
    crate::project_paths::find_source_root(entry)
        .map(Path::to_path_buf)
        .unwrap_or(parent)
}

/// Resolve what `wj run [PATH] [--bin NAME]` should execute.
pub fn resolve_run_target(path: Option<&Path>, cwd: &Path, bin: Option<&str>) -> Result<RunTarget, String> {
    let start = path.map(|p| if p.is_absolute() { p.to_path_buf() } else { cwd.join(p) });
    if let Some(p) = &start {
        if !p.exists() {
            return Err(format!("no such file or directory: {}", path.unwrap().display()));
        }
        if p.is_file() && bin.is_none() {
            let project_root = find_manifest_dir(p);
            return Ok(RunTarget {
                source_root: source_root_for(p, project_root.as_deref()),
                project_root,
                entry: p.clone(),
            });
        }
        if p.is_dir() && bin.is_none() {
            for cand in ["main.wj", "src/main.wj"] {
                let e = p.join(cand);
                if e.is_file() {
                    let project_root = find_manifest_dir(&e);
                    return Ok(RunTarget {
                        source_root: source_root_for(&e, project_root.as_deref()),
                        project_root,
                        entry: e,
                    });
                }
            }
        }
    }
    let from = start.unwrap_or_else(|| cwd.to_path_buf());
    let root = find_manifest_dir(&from).ok_or_else(|| {
        format!(
            "could not find `{}` in {} or any parent directory (pass a .wj file or a directory with main.wj)",
            MANIFEST,
            from.display()
        )
    })?;
    let bins = bin_targets(&root);
    let chosen = match bin {
        Some(name) => bins.iter().find(|b| b.name == name).cloned().ok_or_else(|| {
            format!("no binary named `{}`; available: {}", name, names(&bins))
        })?,
        None => bins
            .first()
            .cloned()
            .ok_or_else(|| format!("no binary in {} (expected src/main.wj, main.wj, [[bin]], cmd/*/main.wj or src/bin/*.wj)", root.display()))?,
    };
    Ok(RunTarget {
        source_root: source_root_for(&chosen.entry, Some(&root)),
        entry: chosen.entry,
        project_root: Some(root),
    })
}

fn names(bins: &[BinTarget]) -> String {
    if bins.is_empty() {
        return "none".into();
    }
    bins.iter().map(|b| b.name.as_str()).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, s: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    #[test]
    fn finds_manifest_upward_and_default_binary() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path().canonicalize().unwrap();
        write(&r, "wj.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\n");
        write(&r, "src/main.wj", "fn main() {}\n");
        write(&r, "src/deep/x.wj", "");
        write(&r, "cmd/tool/main.wj", "fn main() {}\n");
        write(&r, "src/bin/aux.wj", "fn main() {}\n");
        let t = resolve_run_target(None, &r.join("src/deep"), None).unwrap();
        assert_eq!(t.entry, r.join("src/main.wj"));
        assert_eq!(t.source_root, r.join("src"));
        let names: Vec<String> = bin_targets(&r).into_iter().map(|b| b.name).collect();
        assert_eq!(names, vec!["app", "tool", "aux"]);
        let tool = resolve_run_target(None, &r, Some("tool")).unwrap();
        assert_eq!(tool.source_root, r.join("cmd/tool"));
        let aux = resolve_run_target(None, &r, Some("aux")).unwrap();
        assert_eq!(aux.source_root, r.join("src"));
        let by_dir = resolve_run_target(Some(Path::new("cmd/tool")), &r, None).unwrap();
        assert_eq!(by_dir.entry, r.join("cmd/tool/main.wj"));
        assert!(resolve_run_target(None, &r, Some("nope")).unwrap_err().contains("available: app, tool, aux"));
    }

    #[test]
    fn manifest_bins_come_first() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path().canonicalize().unwrap();
        write(&r, "wj.toml", "[package]\nname = \"app\"\n\n[[bin]]\nname = \"srv\"\npath = \"server/main.wj\"\n");
        write(&r, "server/main.wj", "fn main() {}\n");
        write(&r, "src/main.wj", "fn main() {}\n");
        assert_eq!(resolve_run_target(None, &r, None).unwrap().entry, r.join("server/main.wj"));
    }

    #[test]
    fn missing_manifest_is_a_clear_error() {
        let d = tempfile::tempdir().unwrap();
        let e = resolve_run_target(None, d.path(), None).unwrap_err();
        assert!(e.contains("could not find `wj.toml`"), "{e}");
    }
}
