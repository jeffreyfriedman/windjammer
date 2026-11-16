//! The working directory is process-wide, so a test that changes it races every other test in the binary (a
//! removed temp dir under another test's feet is "Could not locate working directory"). Every change goes through
//! `windjammer::test_utils::in_dir`, which holds one lock. This gate scans `tests/` for a bare call and checks
//! itself against planted violations.

use std::path::{Path, PathBuf};

fn needle() -> String {
    ["env", "::", "set_current_", "dir("].concat()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Lines of `src` that change the process working directory (comments and the gate's own text excluded).
fn bare_calls(src: &str) -> Vec<usize> {
    let n = needle();
    src.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with("//") && l.contains(&n))
        .map(|(i, _)| i + 1)
        .collect()
}

#[test]
fn no_test_changes_the_working_directory_outside_the_locked_helper() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    walk(&root.join("tests"), &mut files);
    let mut bad = Vec::new();
    for f in files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if rel == "tests/cwd_lock_gate_test.rs" {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        for line in bare_calls(&text) {
            bad.push(format!("{rel}:{line}"));
        }
    }
    assert!(bad.is_empty(), "use windjammer::test_utils::in_dir (one lock for the binary) instead of a bare chdir:\n  {}", bad.join("\n  "));
}

#[test]
fn the_scan_finds_planted_bare_calls_and_ignores_comments() {
    let n = needle();
    let planted = format!("fn t() {{\n    std::{n}p).unwrap();\n}}\n");
    assert_eq!(bare_calls(&planted), vec![2]);
    let commented = format!("    // std::{n}p)\n");
    assert!(bare_calls(&commented).is_empty());
    assert!(bare_calls("windjammer::test_utils::in_dir(d, || 1)").is_empty());
}
