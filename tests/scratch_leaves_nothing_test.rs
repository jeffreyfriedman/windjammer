//! Gate: a normal (not killed) run of scratch-using tests leaves NOTHING in the temp dir: no `wj-*` scratch, no
//! `.tmp*` from tempfile, no redirected cargo cache root; only `wj`'s own shared caches (`wj-embedded`, `wj-meta`)
//! may be there. A representative set of this binary's own tests runs in a child process whose `TMPDIR` is a private
//! directory; afterwards that directory must be empty. With `WJ_KEEP_TMP=1` it must not be, so the check is not
//! vacuous.

#[path = "common/scratch.rs"]
mod scratch;
use std::process::Command;

/// Deliberate shared caches `wj` itself keeps in the temp dir (embedded runtime, crate metadata): reused across runs,
/// never per-run leftovers (the same list as tests/tmp_hygiene_test.rs).
const WJ_OWN_CACHES: &[&str] = &["wj-embedded", "wj-meta"];

/// Tests that create scratch dirs, private cache roots and child test processes.
const REPRESENTATIVE: &[&str] = &["cargo_cache_active_test::", "toolchain_stale_test::", "stdin_modes_test::"];

fn run_in_private_tmp(filters: &[&str], keep: bool) -> (String, Vec<String>) {
    let tmp = scratch::Scratch::new("leftover-gate");
    let mut c = Command::new(std::env::current_exe().expect("test binary"));
    c.args(filters).arg("--test-threads=2").env("TMPDIR", tmp.path()).env_remove("WJ_KEEP_TMP").env_remove(scratch::CHILD_MARKER);
    if keep {
        c.env("WJ_KEEP_TMP", "1");
    }
    let out = c.output().expect("run test child");
    let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "child run failed:\n{text}");
    let mut left: Vec<String> = std::fs::read_dir(tmp.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !WJ_OWN_CACHES.contains(&n.as_str()))
        .collect();
    left.sort();
    (text, left)
}

fn passed(text: &str) -> usize {
    text.lines()
        .filter_map(|l| l.strip_prefix("test result: ok. "))
        .filter_map(|r| r.split(' ').next()?.parse::<usize>().ok())
        .sum()
}

#[test]
fn a_normal_run_of_scratch_using_tests_leaves_the_temp_dir_empty() {
    let (text, left) = run_in_private_tmp(REPRESENTATIVE, false);
    assert!(passed(&text) >= 8, "too few tests ran: the gate would be vacuous:\n{text}");
    assert!(left.is_empty(), "a normal test run left these in TMPDIR: {left:?}\n{text}");
}

#[test]
fn the_leftover_gate_sees_kept_scratch() {
    let (_, left) = run_in_private_tmp(&["cargo_cache_active_test::"], true);
    assert!(left.iter().any(|n| n.starts_with("wj-cache-")), "WJ_KEEP_TMP=1 kept nothing: the empty check proves nothing: {left:?}");
}
