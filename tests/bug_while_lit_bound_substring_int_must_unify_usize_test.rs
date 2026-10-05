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

//! FAILING REPRO — `while i < 64` (literal int bound) + `strings.substring(text, i, i+1)`
//! emits `while i < 64_usize` with `i: i32` and uncast start index → E0308.
//!
//! Product: `wj-sha::looks_like_sha256_hex` fixed-width hex scan (tip 18:38).
//! Related GREEN gates use `while i < strings.len(...)` (len-driven usize unify);
//! literal bound still poisons the loop var width.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::strings

pub fn first_hex_char(text: string) -> string {
    let mut i = 0
    while i < 64 {
        let ch = strings.substring(text, i, i + 1)
        return ch
    }
    ""
}
"#;

#[test]
fn while_lit_bound_substring_int_must_unify_usize() {
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
    let bad_bound = generated.contains("64_usize") && generated.contains("i < 64_usize");
    assert!(
        !bad_bound,
        "RED P3.679: while i < 64 must not emit 64_usize against i32 i:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "RED P3.679: cargo check failed (int loop bound + substring):\n{}\n{}\ngenerated:\n{generated}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
