//! Windjammer packages named by `path = "../pkg/build"` are built before their dependents: the dependent's
//! `Cargo.toml` links `../pkg/build`, which only exists once `wj build src` ran in the package.
//! Building in dependency order here means a clean checkout builds in any directory order.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const BUILDING_MARKER: &str = ".wj-building";

/// Path dependencies of the project at `project_root` that are Windjammer packages linked through their `build/` output.
pub(crate) fn build_output_dependencies(project_root: &Path) -> Vec<PathBuf> {
    let nodes = crate::package::graph::resolve(project_root).unwrap_or_default();
    nodes
        .into_iter()
        .filter(|n| n.kind == crate::package::graph::DepKind::Windjammer)
        .filter(|n| n.source.strip_prefix("path+").is_some_and(|p| Path::new(p).file_name().is_some_and(|f| f == "build")))
        .filter_map(|n| n.root)
        .collect()
}

fn newest_source(pkg: &Path) -> Option<std::time::SystemTime> {
    let src = if pkg.join("src").is_dir() { pkg.join("src") } else { pkg.to_path_buf() };
    let mut files = crate::module_system::sources::wj_files_under(&src).unwrap_or_default();
    files.push(pkg.join("wj.toml"));
    files.iter().filter_map(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok()).max()
}

/// The package's generated crate is current: its `Cargo.toml` is newer than every source file.
fn is_built(pkg: &Path) -> bool {
    let built = std::fs::metadata(pkg.join("build").join("Cargo.toml")).and_then(|m| m.modified()).ok();
    match (built, newest_source(pkg)) {
        (Some(b), Some(s)) => b >= s,
        (Some(_), None) => true,
        _ => false,
    }
}

/// Build every missing or stale `build/` dependency of `project_root` (each package builds its own first).
pub fn build_missing(project_root: &Path) -> Result<()> {
    for pkg in build_output_dependencies(project_root) {
        let marker = pkg.join("build").join(BUILDING_MARKER);
        // a package already being built further up the chain (a dependency cycle) is left to that build
        if marker.exists() || is_built(&pkg) {
            continue;
        }
        let Ok(exe) = std::env::current_exe() else { continue };
        std::fs::create_dir_all(pkg.join("build"))?;
        std::fs::write(&marker, "")?;
        let src = if pkg.join("src").is_dir() { "src" } else { "." };
        let status = Command::new(exe)
            .args(["build", src, "--no-cargo"])
            .current_dir(&pkg)
            .stdout(std::process::Stdio::null())
            .status();
        let _ = std::fs::remove_file(&marker);
        match status {
            Ok(s) if s.success() => {}
            _ => bail!("dependency {} failed to build (wj build {} in {})", pkg.display(), src, pkg.display()),
        }
    }
    Ok(())
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
    fn only_build_linked_windjammer_packages_are_listed() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        write(r, "app/wj.toml", "[package]\nname = \"app\"\n\n[dependencies]\nwj_x = { path = \"../x/build\" }\nwj_y = { path = \"../y\" }\nraw = { path = \"../raw/build\" }\n");
        write(r, "x/wj.toml", "[package]\nname = \"x\"\n");
        write(r, "x/src/lib.wj", "pub fn f() -> int { 1 }\n");
        write(r, "y/wj.toml", "[package]\nname = \"y\"\n");
        write(r, "y/src/lib.wj", "pub fn g() -> int { 1 }\n");
        write(r, "raw/build/Cargo.toml", "[package]\nname = \"raw\"\n");
        let got = build_output_dependencies(&r.join("app"));
        assert_eq!(got.len(), 1);
        assert!(got[0].ends_with("x"));
        assert!(!is_built(&got[0]));
        write(r, "x/build/Cargo.toml", "[package]\nname = \"x\"\n");
        assert!(is_built(&got[0]));
    }
}
