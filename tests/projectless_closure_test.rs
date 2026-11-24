//! A manifest-less program's startup cost and cache key depend on its module closure (the entry plus the files its
//! `use` / `mod` declarations reach), never on the directories around it (cwd, siblings, a parent holding other repos).
//!
//! Timing-independent: `WJ_META_SCAN_STATS=1` prints one `meta-scan: dir` line per directory listed and one
//! `meta-scan: closure` line per closure file read for the key; both must be the same whatever surrounds the program.

use std::path::{Path, PathBuf};
use std::process::Command;

const HELLO: &str = "fn main() {\n    println(\"hi\")\n}\n";

fn fill(dir: &Path, files: usize, nested: usize) {
    for n in 0..nested {
        std::fs::create_dir_all(dir.join(format!("d{n}/e"))).unwrap();
    }
    for i in 0..files {
        let d = if nested > 0 { dir.join(format!("d{}", i % nested)) } else { dir.to_path_buf() };
        std::fs::write(d.join(format!("m{i}.wj")), "").unwrap();
    }
}

/// (directories listed, closure files read) by `wj <cmd> file`, run from `cwd`.
fn counts(cmd: &str, file: &Path, cwd: &Path) -> (usize, usize) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_wj"));
    if cmd == "run" { c.args(["run", "--interpret"]) } else { c.arg(cmd) }.arg(file).current_dir(cwd);
    let o = c.env("WJ_META_SCAN_STATS", "1").output().expect("run wj");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "wj {cmd} failed:\n{err}");
    let n = |k: &str| err.lines().filter(|l| *l == format!("meta-scan: {k}")).count();
    (n("dir"), n("closure"))
}

/// `<tmp>/src/prog/hello.wj` with `siblings` big directories beside `prog` under the `src` ancestor (the layout that
/// made the old walk start at `src`, the parent of other repos).
fn hello_beside_siblings(siblings: usize) -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let src = t.path().join("src");
    std::fs::create_dir_all(src.join("prog")).unwrap();
    std::fs::write(src.join("prog/hello.wj"), HELLO).unwrap();
    for s in 0..siblings {
        let d = src.join(format!("repo{s}"));
        std::fs::create_dir_all(&d).unwrap();
        fill(&d, 2000, 50);
    }
    let f = src.join("prog/hello.wj");
    (t, f)
}

#[test]
fn directory_reads_do_not_depend_on_sibling_directories() {
    for cmd in ["check", "run"] {
        let (_a, fa) = hello_beside_siblings(0);
        let (_b, fb) = hello_beside_siblings(3);
        let a = counts(cmd, &fa, fa.parent().unwrap());
        let b = counts(cmd, &fb, fb.parent().unwrap());
        assert_eq!(a, b, "wj {cmd}: (dirs listed, closure files) {a:?} with no siblings, {b:?} with 3 big sibling repos");
    }
}

#[test]
fn hello_costs_the_same_among_2000_unrelated_files() {
    for cmd in ["check", "run"] {
        let alone = tempfile::tempdir().unwrap();
        let busy = tempfile::tempdir().unwrap();
        for d in [&alone, &busy] {
            std::fs::write(d.path().join("hello.wj"), HELLO).unwrap();
        }
        fill(busy.path(), 2000, 20);
        let a = counts(cmd, &alone.path().join("hello.wj"), alone.path());
        let b = counts(cmd, &busy.path().join("hello.wj"), busy.path());
        assert_eq!(a.1, 1, "wj {cmd}: the closure of hello.wj is itself ({a:?})");
        assert_eq!(a.1, b.1, "wj {cmd}: closure files read {} alone, {} among 2000 unrelated files", a.1, b.1);
        assert_eq!(a.0, b.0, "wj {cmd}: directories listed {} alone, {} among 2000 unrelated files", a.0, b.0);
    }
}

#[test]
fn key_follows_the_closure_and_ignores_unrelated_siblings() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    std::fs::write(d.join("main.wj"), "use util::f\nmod shapes;\nfn main() {}\n").unwrap();
    std::fs::write(d.join("util.wj"), "pub fn f() {}\n").unwrap();
    std::fs::create_dir_all(d.join("shapes")).unwrap();
    std::fs::write(d.join("shapes/mod.wj"), "pub fn g() {}\n").unwrap();
    let key = || windjammer::metadata::projectless_program_key(&d.join("main.wj"));
    let k0 = key();
    // an unrelated sibling file, and one in an unrelated directory: the key stays
    std::fs::write(d.join("unrelated.wj"), "fn z() {}\n").unwrap();
    std::fs::create_dir_all(d.join("other")).unwrap();
    std::fs::write(d.join("other/x.wj"), "fn y() {}\n").unwrap();
    assert_eq!(key(), k0, "an unrelated sibling changed the key");
    // an edit to a closure file (imported by `use`, then declared by `mod`) changes it
    std::fs::write(d.join("util.wj"), "pub fn f() { let a = 1 }\n").unwrap();
    let k1 = key();
    assert_ne!(k1, k0, "editing an imported file left the key stale");
    std::fs::write(d.join("shapes/mod.wj"), "pub fn g() { let b = 22 }\n").unwrap();
    assert_ne!(key(), k1, "editing a `mod` file left the key stale");
}
