#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! P3.528: `version_applied(applied: Vec<int>, …)` only reads via `for v in applied`.
//! Call sites that reuse `applied` across a loop must borrow (`&applied`), not move.
//! Product `wj-migrate` `apply_conn` moves; `pending` already borrows. Do not reshape.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
pub fn version_applied(applied: Vec<int>, version: int) -> bool {
    for v in applied {
        if v == version {
            return true
        }
    }
    false
}

pub fn apply_like(applied: Vec<int>, located: Vec<int>) -> int {
    let mut n = 0
    for item in located {
        if version_applied(applied, item) {
            continue
        }
        n = n + 1
    }
    n
}
"#;

fn moves_applied(rs: &str) -> bool {
    rs.lines().any(|l| {
        let t = l.trim();
        t.contains("version_applied(applied,") && !t.contains("applied.clone()")
    })
}

#[test]
fn copy_elem_vec_for_in_callee_must_borrow_at_loop_call_sites() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), SOURCE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.528 isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.528 isolate emit:\n{rs}");

    // Signature layer: Copy-element Vec for-in should demote to &Vec
    assert!(
        rs.contains("version_applied(applied: &Vec<")
            || rs.contains("version_applied(applied: &[")
            || rs.contains("fn version_applied(applied: &Vec"),
        "P3.528: version_applied formal must be borrowed &Vec (Copy elems):\n{rs}"
    );
    assert!(
        !moves_applied(&rs),
        "P3.528 RED: loop call site must not move applied into version_applied:\n{rs}"
    );
    assert!(
        rs.contains("version_applied(&applied,") || rs.contains("version_applied(applied,"),
        "P3.528: expected borrowed call site:\n{rs}"
    );
    // If formal is &Vec, bare `applied` at call site is also OK when applied is already &Vec —
    // but apply_like takes owned Vec and must pass &applied.
    if rs.contains("fn apply_like(applied: Vec") || rs.contains("apply_like(applied: Vec") {
        assert!(
            rs.contains("version_applied(&applied,"),
            "P3.528: owned apply_like.applied must borrow into demoted callee:\n{rs}"
        );
    }
}
