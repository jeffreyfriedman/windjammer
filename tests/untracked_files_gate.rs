//! Portability gate: the test suite must leave no untracked files in the repository (tests write to temp
//! dirs, see tests/common/scratch.rs). Untracked files trip the span_oracle ratchets and make results depend
//! on which tests ran before. Run it AFTER the suite: `cargo test --test untracked_files_gate`.
//! Ignored files count too under `tests/` (e.g. the old `tests/generated/`); build output elsewhere is fine.

use std::path::Path;
use std::process::Command;

fn git_lines(repo: &Path, args: &[&str]) -> Vec<String> {
    let out = Command::new("git").current_dir(repo).args(args).output().expect("run git");
    assert!(out.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect()
}

/// Untracked (not ignored) files anywhere, plus ignored files under `tests/`.
pub fn stray_files(repo: &Path) -> Vec<String> {
    let mut v = git_lines(repo, &["ls-files", "--others", "--exclude-standard"]);
    v.extend(git_lines(repo, &["ls-files", "--others", "--ignored", "--exclude-standard", "--", "tests"]));
    v.sort();
    v.dedup();
    v
}

#[test]
fn the_suite_leaves_no_untracked_files() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    if !repo.join(".git").exists() {
        return; // a source archive, not a checkout
    }
    let stray = stray_files(repo);
    assert!(
        stray.is_empty(),
        "{} untracked file(s) in the repository (a test wrote into the checkout instead of a temp dir, or a new file is uncommitted):\n{}",
        stray.len(),
        stray.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn detector_sees_untracked_and_ignored_test_output() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    let run = |a: &[&str]| assert!(Command::new("git").current_dir(r).args(a).output().unwrap().status.success());
    run(&["init", "-q"]);
    std::fs::create_dir_all(r.join("tests/generated")).unwrap();
    std::fs::write(r.join(".gitignore"), "tests/generated/\n").unwrap();
    run(&["add", ".gitignore"]);
    assert!(stray_files(r).is_empty());
    std::fs::write(r.join("tests/temp_x.wj"), "fn main() {}\n").unwrap();
    std::fs::write(r.join("tests/generated/out.rs"), "").unwrap();
    let got = stray_files(r);
    assert!(got.contains(&"tests/temp_x.wj".to_string()), "{got:?}");
    assert!(got.contains(&"tests/generated/out.rs".to_string()), "{got:?}");
}
