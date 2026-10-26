//! The interpreter-primary readiness scorecard (scripts/interp_readiness.py) is itself under test: its
//! python unit tests drive the criteria logic with a fake `wj` shim (known pass/fail/divergence patterns)
//! and include a mutation test (an always-pass `wj` must be flagged SUSPECT by the cross-checks).

use std::path::Path;
use std::process::Command;

#[test]
fn readiness_scorecard_logic_tests_pass() {
    if Command::new("python3").arg("--version").output().is_err() {
        eprintln!("python3 not available: skipping scorecard tests");
        return;
    }
    let scripts = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
    let out = Command::new("python3")
        .args(["-W", "ignore", "-m", "unittest", "discover", "-s", "readiness/tests", "-t", ".", "-p", "test_*.py"])
        .current_dir(&scripts)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "scorecard tests failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
