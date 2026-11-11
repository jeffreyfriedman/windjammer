//! `wj run --interpret --speculate` (ADR-0033 option a, opt-in): a bounded background compiled build. The bounds are
//! checked directly (`cli::speculate::skip_reason` against a scratch cache root); the run itself must never change the
//! program's stdout or exit status and must not wait for the build.

use std::path::Path;
use windjammer::cli::{speculate, verdict};

fn program(dir: &Path) -> std::path::PathBuf {
    let f = dir.join("s.wj");
    std::fs::write(&f, "fn main() {\n    println(\"spec\")\n}\n").unwrap();
    f
}

#[test]
fn a_recorded_verdict_means_no_new_build() {
    let d = tempfile::tempdir().unwrap();
    let f = program(d.path());
    let root = d.path().join("cache");
    let key = verdict::program_key(&f).unwrap();
    verdict::store_in(&root.join("verdicts"), &verdict::Verdict::new(&key, verdict::Outcome::Accepted, false), 8);
    let why = speculate::skip_reason(&f, &root).expect("skipped");
    assert!(why.contains("already recorded"), "{why}");
}

#[test]
fn a_live_speculative_build_is_never_doubled_and_a_dead_one_is_ignored() {
    let d = tempfile::tempdir().unwrap();
    let f = program(d.path());
    let root = d.path().join("cache");
    std::fs::create_dir_all(&root).unwrap();
    // this test process stands in for a running build
    std::fs::write(root.join("speculative.pid"), std::process::id().to_string()).unwrap();
    assert_eq!(speculate::running(&root), Some(std::process::id()));
    let why = speculate::skip_reason(&f, &root).expect("skipped while one runs");
    assert!(why.contains("still running"), "{why}");
    // a finished process: spawn one and wait for it, then its pid no longer counts
    let mut c = std::process::Command::new("true").spawn().unwrap();
    let pid = c.id();
    c.wait().unwrap();
    std::fs::write(root.join("speculative.pid"), pid.to_string()).unwrap();
    assert_eq!(speculate::running(&root), None);
}

#[test]
fn a_speculative_run_prints_what_the_program_prints_and_does_not_wait() {
    let d = tempfile::tempdir().unwrap();
    let f = program(d.path());
    let wj = env!("CARGO_BIN_EXE_wj");
    let start = std::time::Instant::now();
    let o = std::process::Command::new(wj).args(["run", "--interpret", "--speculate"]).arg(&f).output().unwrap();
    let took = start.elapsed();
    assert_eq!(String::from_utf8_lossy(&o.stdout), "spec\n");
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
    // a compiled build of the runtime takes far longer; the interpreted run returns without waiting for it
    assert!(took < std::time::Duration::from_secs(20), "the run waited {took:?}");
}

#[test]
fn speculate_is_rejected_with_compiled() {
    let d = tempfile::tempdir().unwrap();
    let f = program(d.path());
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "--compiled", "--speculate"]).arg(&f).output().unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stderr).contains("cannot be used with"));
}
