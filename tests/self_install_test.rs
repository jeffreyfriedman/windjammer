//! Gate: `wj self-install` works from an installed binary (no source checkout nearby) and installs
//! the standard library embedded in the binary; `--dry-run` writes nothing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(d: &Path, root: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, root, out);
            } else {
                out.push(p.strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    walk(root, root, &mut out);
    out.sort();
    out
}

/// A copy of the `wj` binary in a directory that has no `Cargo.toml`/`src` above it.
fn detached_wj(dir: &Path) -> PathBuf {
    let dest = dir.join("wj-detached");
    fs::copy(WJ, &dest).unwrap();
    dest
}

#[test]
fn dry_run_writes_nothing_and_reports_the_plan() {
    let home = TempDir::new().unwrap();
    let bindir = TempDir::new().unwrap();
    let exe = detached_wj(bindir.path());
    let o = Command::new(&exe).args(["self-install", "--dry-run"]).env("HOME", home.path()).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("would install"), "{out}");
    assert!(out.contains("stdlib"), "{out}");
    assert!(files_under(home.path()).is_empty(), "dry run must not create files: {:?}", files_under(home.path()));
}

#[test]
fn installed_binary_installs_the_embedded_stdlib() {
    let home = TempDir::new().unwrap();
    let bindir = TempDir::new().unwrap();
    let exe = detached_wj(bindir.path());
    let o = Command::new(&exe).arg("self-install").env("HOME", home.path()).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let installed = home.path().join(".wj/std");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("std");
    let want = files_under(&source);
    let got = files_under(&installed);
    assert!(!want.is_empty());
    assert_eq!(got, want, "installed std/ must be exactly the embedded std/ tree");
    for rel in &want {
        assert_eq!(fs::read(installed.join(rel)).unwrap(), fs::read(source.join(rel)).unwrap(), "{}", rel.display());
    }
    assert!(home.path().join(".wj/bin/wj").is_file());
    // a second install replaces rather than merges: a stale file from an older std/ disappears
    fs::write(installed.join("stale_removed_file.wj"), "x").unwrap();
    let o = Command::new(&exe).arg("self-install").env("HOME", home.path()).output().unwrap();
    assert!(o.status.success());
    assert!(!installed.join("stale_removed_file.wj").exists());
}
