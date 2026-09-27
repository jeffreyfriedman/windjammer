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

//! If-else int literals assigned to `let status` must coerce into `status: u16`.
//!
//! Distinct from `server_response_new_int_literal_must_coerce_to_u16` (call-site
//! literals / `status: int` formal). Ecosystem `wj-notes-api` PUT error arm:
//! `let status = if msg == "note not found" { 404 } else { 400 }` then
//! `error_from_message(status, msg)` emits `404_i32` / `400_i32` (E0308).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn if_int_status_into_u16_formal_must_coerce() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::write(
        &src,
        r#"
fn error_from_message(status: u16, message: string) -> u16 {
    status
}

pub fn put_error(msg: string) -> u16 {
    let status = if msg == "note not found" { 404 } else { 400 }
    error_from_message(status, msg)
}
"#,
    )
    .unwrap();

    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("build");
    assert!(
        build.status.success(),
        "isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("t.rs"))
        .or_else(|_| fs::read_to_string(out.join("main.rs")))
        .or_else(|_| fs::read_to_string(out.join("lib.rs")))
        .unwrap_or_else(|_| {
            let mut acc = String::new();
            if let Ok(entries) = fs::read_dir(&out) {
                for e in entries.flatten() {
                    if e.path().extension().and_then(|s| s.to_str()) == Some("rs") {
                        acc.push_str(&fs::read_to_string(e.path()).unwrap_or_default());
                    }
                }
            }
            acc
        });
    eprintln!("generated:\n{rs}");

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "if-int status into u16 formal must cargo-check.\n{rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
