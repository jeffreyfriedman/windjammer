#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! Repository scripts that were ported from Python/shell to interpreted Windjammer (`scripts/wj/*.wj`) stay
//! pinned to their originals: `scripts/wj/port_gates.wj` runs every case under `tests/py_ports/` and requires the
//! port's stdout, stderr, exit code and output files to equal what the original produced (recorded in the case).
//!
//! The ports use packages from windjammer-ecosystem, checked out (or symlinked) at the repository root as
//! `windjammer-ecosystem/`. Without it the tests skip and say so.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ecosystem_present() -> bool {
    if root().join("windjammer-ecosystem/packages").is_dir() {
        return true;
    }
    eprintln!("SKIPPED: windjammer-ecosystem/ is not checked out at the repository root");
    false
}

fn gate(cases: &Path, extra: &[&str]) -> Output {
    Command::new(WJ)
        .args(["run", "scripts/wj/port_gates.wj", "--wj", WJ, "--cases"])
        .arg(cases)
        .args(extra)
        .current_dir(root())
        .output()
        .unwrap()
}

#[test]
fn every_port_matches_its_recorded_original() {
    if !ecosystem_present() {
        return;
    }
    let out = gate(&root().join("tests/py_ports"), &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "port gates failed:\n{stdout}\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("port gate cases passed"), "{stdout}");
}

/// A gate that cannot fail proves nothing: damage one recorded expectation and the gate must say FAIL.
#[test]
fn a_damaged_expectation_fails_the_gate() {
    if !ecosystem_present() {
        return;
    }
    let tmp = tempfile::TempDir::new().unwrap();
    let src = root().join("tests/py_ports");
    fs::copy(src.join("ORIGIN"), tmp.path().join("ORIGIN")).unwrap();
    let case = tmp.path().join("check_versions/consistent");
    fs::create_dir_all(&case).unwrap();
    for f in ["cmd", "cmd.record", "expected.stdout", "expected.stderr", "expected.code"] {
        fs::copy(src.join("check_versions/consistent").join(f), case.join(f)).unwrap();
    }
    copy_dir(&src.join("check_versions/consistent/tree"), &case.join("tree"));
    let ok = gate(tmp.path(), &[]);
    assert!(ok.status.success(), "undamaged copy must pass:\n{}", String::from_utf8_lossy(&ok.stdout));
    let want = fs::read_to_string(case.join("expected.stdout")).unwrap();
    fs::write(case.join("expected.stdout"), want.replace("Workspace version", "Workspace  version")).unwrap();
    let bad = gate(tmp.path(), &[]);
    assert!(!bad.status.success(), "a damaged expectation must fail the gate");
    assert!(String::from_utf8_lossy(&bad.stdout).contains("FAIL check_versions/consistent"));
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dest = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            fs::copy(e.path(), dest).unwrap();
        }
    }
}
