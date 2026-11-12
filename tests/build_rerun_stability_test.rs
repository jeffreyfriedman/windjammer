//! A build script rerun must be cheap: no test output file, and no `git status`, may cost a library recompile.
//!
//! Measured (cargo -v): "Dirty windjammer: the file `tests` has changed". A build-script rerun recompiles the
//! whole library (~4-8 min), and build.rs reran whenever anything under a watched path changed: the test
//! suite writes files under `tests/` (tests/generated, per-project .wj-cache dirs), and `git status` rewrites
//! `.git/index`, which build.rs also watched. Gates here pin the fixes: git queries never rewrite the index,
//! generated files are written only on change, build.rs does not watch the index or all of `tests/`, and a
//! test file the build has not seen yet is reported loudly.

#[path = "../build_support/stable_build.rs"]
mod stable_build;
#[path = "../build_support/test_modules.rs"]
mod test_modules;

use std::path::Path;
use std::time::{Duration, SystemTime};

fn mtime(p: &Path) -> SystemTime {
    std::fs::metadata(p).unwrap().modified().unwrap()
}

/// Backdate a file so an mtime change is visible regardless of filesystem timestamp resolution.
fn backdate(p: &Path) {
    let f = std::fs::OpenOptions::new().write(true).open(p).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(3600)).unwrap();
}

#[test]
fn unchanged_generated_content_keeps_the_file_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("generated.rs");
    assert!(stable_build::write_if_changed(&p, "pub const A: u8 = 1;\n").unwrap(), "first write creates the file");
    backdate(&p);
    let before = mtime(&p);
    assert!(!stable_build::write_if_changed(&p, "pub const A: u8 = 1;\n").unwrap());
    assert_eq!(mtime(&p), before, "identical content must not bump the mtime (it would recompile the library)");
    assert!(stable_build::write_if_changed(&p, "pub const A: u8 = 2;\n").unwrap(), "changed content is written");
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "pub const A: u8 = 2;\n");
}

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    let out = std::process::Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    out
}

/// A repo whose index is stale (a tracked file has a newer stat than the index recorded), which is the
/// state in which a plain `git status` refreshes and rewrites `.git/index`.
fn repo_with_stale_index() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("a.txt"), "one\n").unwrap();
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "base"]);
    // Same content, new mtime: status must re-hash it and (optionally) refresh the index entry.
    std::thread::sleep(Duration::from_millis(1100));
    std::fs::write(root.join("a.txt"), "one\n").unwrap();
    let index = root.join(".git/index");
    backdate(&index);
    (dir, index)
}

#[test]
fn build_script_git_queries_do_not_rewrite_the_index() {
    let (dir, index) = repo_with_stale_index();
    let before = mtime(&index);
    for args in [&["status", "--porcelain", "--untracked-files=no"][..], &["diff-index", "-p", "HEAD"][..]] {
        let out = stable_build::git_command(dir.path(), args).output().unwrap();
        assert!(out.status.success());
    }
    assert_eq!(mtime(&index), before, "the build script's git calls rewrote .git/index, which makes every build dirty");
}

/// Mutation witness: the same query without `--no-optional-locks` does rewrite the index, so the gate above
/// can fail (if this ever stops holding, git changed and the gate needs a new witness).
#[test]
fn plain_git_status_does_rewrite_a_stale_index() {
    let (dir, index) = repo_with_stale_index();
    let before = mtime(&index);
    git(dir.path(), &["status", "--porcelain", "--untracked-files=no"]);
    assert_ne!(mtime(&index), before, "plain `git status` no longer refreshes the index; update the witness");
}

/// `git diff HEAD` is not a safe substitute: it rewrites a stale index even under `--no-optional-locks`.
#[test]
fn plain_git_diff_head_rewrites_a_stale_index_even_without_optional_locks() {
    let (dir, index) = repo_with_stale_index();
    let before = mtime(&index);
    stable_build::git_command(dir.path(), &["diff", "HEAD"]).output().unwrap();
    assert_ne!(mtime(&index), before, "`git diff HEAD` no longer rewrites the index; build.rs could use it again");
}

#[test]
fn build_rs_uses_the_stable_helpers_and_does_not_watch_the_index() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs")).unwrap();
    assert!(!src.contains("\"index\""), "build.rs must not rerun on .git/index (git status rewrites it)");
    assert!(!src.contains("\"diff\", \"HEAD\""), "`git diff HEAD` rewrites a stale index; use diff-index -p HEAD");
    assert!(!src.contains("Command::new(\"git\")"), "build.rs must run git through stable_build::git_command");
    for line in src.lines().filter(|l| l.contains("fs::write(")) {
        assert!(line.contains("stamp"), "build.rs writes `{}` unconditionally; use stable_build::write_if_changed", line.trim());
    }
}

/// build.rs no longer watches `tests/` as a whole (test output files made every build dirty), so a test
/// file added after the last build-script run is not in the `all` binary yet. This is the loud signal.
#[test]
fn every_test_file_is_part_of_the_generated_all_binary() {
    const GENERATED: &str = include_str!(concat!(env!("OUT_DIR"), "/all_tests_generated.rs"));
    let tests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let missing: Vec<String> = test_modules::discover_test_modules(&tests_dir)
        .unwrap()
        .into_iter()
        .filter(|m| !GENERATED.contains(&format!("/tests/{}\"", m.rel_path)))
        .map(|m| format!("tests/{}", m.rel_path))
        .collect();
    assert!(
        missing.is_empty(),
        "{missing:?} not in the `all` test binary yet. cargo only rescans tests/ when build.rs or a known test \
         file changes (watching the whole directory made every test run dirty the next build). Run `touch build.rs` once."
    );
}
