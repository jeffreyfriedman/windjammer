//! A single-file build reads the same amount of metadata whatever directory tree surrounds the file
//! (location-agnostic rule; docs/known-issues/build-scans-enclosing-tree-for-metadata.md; docs/economics C12).
//!
//! Timing-independent: `WJ_META_SCAN_STATS=1` makes `wj build` print how many directories its metadata merge listed
//! and how many `.wj.meta` files it opened. The same program is built inside a project whose cache holds analysis
//! metadata of 2 or 40 other modules the program never names (`.wj-cache/areaN/pkgI/modI.wj.meta`); the files opened
//! must not grow with them. (Directories are still listed to find the peers by name; that count is reported, not
//! gated.)

use std::path::Path;
use std::process::Command;

const PROGRAM: &str = "fn gcd(a: int, b: int) -> int {\n    if b == 0 { a } else { gcd(b, a % b) }\n}\n\nfn main() {\n    println(\"${gcd(12, 18)}\")\n}\n";

/// A project (`wj.toml` at the root) whose cache holds metadata of `junk` modules the program does not name, and the
/// program at `tools/econ/prog/main.wj`.
fn project(root: &Path, junk: usize) -> std::path::PathBuf {
    std::fs::write(root.join("wj.toml"), "[package]\nname = \"p\"\nversion = \"0.1.0\"\n").unwrap();
    for i in 0..junk {
        let d = root.join(".wj-cache").join(format!("area{}", i % 4)).join(format!("pkg{i}"));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(format!("mod{i}.wj.meta")), "{}").unwrap();
        let s = root.join("src").join(format!("area{}", i % 4)).join(format!("pkg{i}"));
        std::fs::create_dir_all(&s).unwrap();
        std::fs::write(s.join(format!("mod{i}.wj")), "pub fn other() {}\n").unwrap();
    }
    let prog = root.join("tools").join("econ").join("prog");
    std::fs::create_dir_all(&prog).unwrap();
    let file = prog.join("main.wj");
    std::fs::write(&file, PROGRAM).unwrap();
    file
}

fn scan_counts(file: &Path, out: &Path) -> (usize, usize) {
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--no-cargo", "--no-lint", "-o"])
        .arg(out)
        .arg(file)
        .env("WJ_META_SCAN_STATS", "1")
        .output()
        .expect("run wj build");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(o.status.success(), "wj build failed:\n{err}");
    let line = err.lines().find(|l| l.starts_with("meta-scan: ")).unwrap_or_else(|| panic!("no meta-scan line in:\n{err}"));
    let num = |k: &str| -> usize {
        line.split_whitespace().find_map(|w| w.strip_prefix(k)).and_then(|v| v.parse().ok()).unwrap()
    };
    (num("dirs="), num("meta_files="))
}

#[test]
fn single_file_build_metadata_reads_do_not_grow_with_the_surrounding_tree() {
    let small = tempfile::tempdir().unwrap();
    let big = tempfile::tempdir().unwrap();
    let f_small = project(small.path(), 2);
    let f_big = project(big.path(), 40);
    let (_, files_small) = scan_counts(&f_small, &small.path().join("out"));
    let (_, files_big) = scan_counts(&f_big, &big.path().join("out"));
    assert_eq!(
        files_small, files_big,
        "a single-file build opened {files_small} metadata files in a project caching 2 unrelated modules but \
         {files_big} with 40: it reads metadata of modules it never names"
    );
}

#[test]
fn a_file_in_a_project_opens_no_more_metadata_than_a_file_alone() {
    let alone = tempfile::tempdir().unwrap();
    let f_alone = alone.path().join("main.wj");
    std::fs::write(&f_alone, PROGRAM).unwrap();
    let big = tempfile::tempdir().unwrap();
    let f_big = project(big.path(), 40);
    let (_, files_alone) = scan_counts(&f_alone, &alone.path().join("out"));
    let (_, files_big) = scan_counts(&f_big, &big.path().join("out"));
    assert_eq!(files_big, files_alone, "metadata files opened: {files_big} in a project vs {files_alone} alone");
}
