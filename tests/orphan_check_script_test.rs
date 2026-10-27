//! `scripts/wj/check_orphan_tests.wj` fails on test files no runner reaches.
//!
//! The script runs interpreted from the repository checkout against a throwaway git repository. It uses packages from
//! windjammer-ecosystem, checked out (or symlinked) at the repository root; without it the tests skip and say so.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/wj/check_orphan_tests.wj")
}

fn ecosystem_present() -> bool {
    let present = Path::new(env!("CARGO_MANIFEST_DIR")).join("windjammer-ecosystem/packages").is_dir();
    if !present {
        eprintln!("SKIPPED: windjammer-ecosystem/ is not checked out at the repository root");
    }
    present
}

fn repo_with(files: &[(&str, &str)]) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    for (rel, body) in files {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    let git = |args: &[&str]| {
        let st = Command::new("git").args(args).current_dir(root).output().unwrap();
        assert!(st.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&st.stderr));
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    tmp
}

fn run(root: &Path) -> (bool, String) {
    let grant = format!("{}/**", root.canonicalize().unwrap().display());
    let out = Command::new(WJ)
        .args(["run", "--allow", &format!("fs.read={grant}"), "--allow", &format!("fs.write={grant}")])
        .arg(script())
        .current_dir(root)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

#[test]
fn unreferenced_wj_file_is_an_orphan() {
    if !ecosystem_present() {
        return;
    }
    let repo = repo_with(&[("tests/lonely.wj", "fn main() {}\n"), ("tests/a_test.rs", "")]);
    let (ok, text) = run(repo.path());
    assert!(!ok && text.contains("tests/lonely.wj"), "{text}");
}

#[test]
fn file_named_by_a_runner_is_reached() {
    if !ecosystem_present() {
        return;
    }
    let repo = repo_with(&[
        ("tests/known.wj", "fn main() {}\n"),
        ("tests/runner_test.rs", "const F: &str = \"known.wj\";\n"),
    ]);
    let (ok, text) = run(repo.path());
    assert!(ok, "{text}");
}

#[test]
fn consumes_glob_reaches_files_in_one_directory_only() {
    if !ecosystem_present() {
        return;
    }
    let repo = repo_with(&[
        ("tests/corpus/a.wj", ""),
        ("tests/corpus/deep/b.wj", ""),
        ("tests/runner_test.rs", "//! consumes: tests/corpus/*.wj\n"),
    ]);
    let (ok, text) = run(repo.path());
    assert!(!ok, "{text}");
    assert!(text.contains("tests/corpus/deep/b.wj") && !text.contains("corpus/a.wj"), "{text}");
}

#[test]
fn allow_list_entries_need_a_reason() {
    if !ecosystem_present() {
        return;
    }
    let repo = repo_with(&[
        ("tests/fixture.wj", ""),
        ("tests/ORPHAN_ALLOWLIST.txt", "tests/fixture.wj\n"),
    ]);
    let (ok, text) = run(repo.path());
    assert!(!ok && text.contains("without a reason"), "{text}");

    let repo = repo_with(&[
        ("tests/fixture.wj", ""),
        ("tests/ORPHAN_ALLOWLIST.txt", "tests/fixture.wj | data for a manual tool\n"),
    ]);
    let (ok, text) = run(repo.path());
    assert!(ok, "{text}");
}

#[test]
fn disabled_test_files_always_fail() {
    if !ecosystem_present() {
        return;
    }
    for name in ["tests/x_test.rs.skip", "tests/y_test.rs.disabled", "crates/c/tests/z.rs.bak"] {
        let repo = repo_with(&[(name, "#[test] fn t() {}")]);
        let (ok, text) = run(repo.path());
        assert!(!ok && text.contains("disabled"), "{name}: {text}");
    }
}
