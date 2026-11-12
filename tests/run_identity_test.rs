//! Pilot 6 finding 1 (`docs/drift/pilot6-repros/same_dir_name_runs_other_binary.md`): two projects whose directories
//! share a name ran each other's binary through `wj run --compiled` (and `--verify`), because every generated crate
//! builds into the shared target directory, where cargo identified the root package by name and version only.
//! The identity of a generated crate now names its own directory (`cargo_cache::crate_version`).

use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// `x/app/main.wj` and `y/app/main.wj`: the two repro programs under one directory name.
fn same_named_projects(root: &Path) -> (PathBuf, PathBuf) {
    let repros = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/drift/pilot6-repros");
    let mut out = Vec::new();
    for (side, src) in [("x", "same_dir_name_x"), ("y", "same_dir_name_y")] {
        let dir = root.join(side).join("app");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(repros.join(src).join("main.wj"), dir.join("main.wj")).unwrap();
        out.push(dir);
    }
    (out[0].clone(), out[1].clone())
}

fn last_line(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stdout).lines().last().unwrap_or("").to_string()
}

#[test]
fn crate_identity_names_the_directory_not_its_leaf_name() {
    let t = tempfile::tempdir().unwrap();
    let (x, y) = same_named_projects(t.path());
    let (vx, vy) = (windjammer::cargo_cache::crate_version(&x), windjammer::cargo_cache::crate_version(&y));
    assert_ne!(vx, vy, "same leaf name, different directories");
    assert_eq!(vx, windjammer::cargo_cache::crate_version(&x.join(".").join("")), "one directory, one identity");
    assert!(vx.starts_with("0.1.0+"), "a version requirement of 0.1.0 still matches: {vx}");
    // the generated manifests of the two builds differ in exactly that identity
    for d in [&x, &y] {
        let o = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(d.join("build")).arg(d.join("main.wj")).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
    let manifest = |d: &Path| std::fs::read_to_string(d.join("build/Cargo.toml")).unwrap();
    let version = |m: String| m.lines().find(|l| l.starts_with("version = ")).unwrap_or("").to_string();
    assert_ne!(version(manifest(&x)), version(manifest(&y)));
}

#[test]
fn compiled_runs_of_same_named_projects_run_their_own_program() {
    let t = tempfile::tempdir().unwrap();
    let (x, y) = same_named_projects(t.path());
    let mut seen = Vec::new();
    for d in [&x, &y, &x, &y] {
        let o = Command::new(WJ).args(["run", "--compiled", "main.wj"]).current_dir(d).stdin(std::process::Stdio::null()).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        seen.push(last_line(&o));
    }
    assert_eq!(seen, ["program X", "program Y", "program X", "program Y"], "an unchanged project ran another project's binary");
}

#[test]
fn builds_of_same_named_projects_publish_their_own_binary() {
    let t = tempfile::tempdir().unwrap();
    let (x, y) = same_named_projects(t.path());
    let mut seen = Vec::new();
    for d in [&x, &y, &x] {
        let o = Command::new(WJ).args(["build", "--no-lint", "-o"]).arg(d.join("build")).arg(d.join("main.wj")).output().unwrap();
        assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        let bin = std::fs::read_dir(d.join("build/target/debug"))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.is_file() && p.extension().is_none())
            .expect("a published binary");
        seen.push(last_line(&Command::new(&bin).output().unwrap()));
    }
    assert_eq!(seen, ["program X", "program Y", "program X"], "`wj build` published another project's binary");
}
