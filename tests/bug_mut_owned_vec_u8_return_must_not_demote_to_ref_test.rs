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

//! FAILING REPRO — `mut out: Vec<u8>` formal demoted to `&Vec<u8>` then returned owned.
//!
//! Ecosystem `wj-uuid` `append_bytes`:
//! ```ignore
//! fn append_bytes(mut out: Vec<u8>, extra: Vec<u8>) -> Vec<u8> {
//!     for byte in extra { out.push(byte) }
//!     out
//! }
//! ```
//! Tip emits `fn append_bytes(out: &Vec<u8>, …) -> Vec<u8> { …; out }` → E0308.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
fn append_bytes(mut out: Vec<u8>, extra: Vec<u8>) -> Vec<u8> {
    for byte in extra {
        out.push(byte)
    }
    out
}

pub fn join_bytes(a: Vec<u8>, b: Vec<u8>) -> Vec<u8> {
    append_bytes(a, b)
}
"#;

#[test]
fn mut_owned_vec_u8_return_must_not_demote_to_ref() {
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
        "library build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    assert!(
        !generated.contains("out: &Vec<u8>")
            && !generated.contains("out: &Vec<")
            && generated.contains("-> Vec<u8>"),
        "RED P3.298: mut owned Vec<u8> formal must stay owned when returned:\n{generated}"
    );

    // cargo-check the generated crate
    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (owned mut Vec return):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
