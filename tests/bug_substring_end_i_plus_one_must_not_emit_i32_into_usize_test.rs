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

//! FAILING REPRO — `strings.substring(s, i, i + 1)` under `while i < strings.len(s)`
//! emits `(i + 1_i32) as usize` → E0277/E0308 (`wj-duration` parse_ms).
//!
//! Related gates (`int_increment_literal_*`, haystack substring) can GREEN on
//! shallower fixtures; this is the product scanner shape.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::strings

fn is_digit(ch: string) -> bool {
    ch >= "0" && ch <= "9"
}

pub fn scan_digits(text: string) -> int {
    let mut i = 0
    let mut n = 0
    while i < strings.len(text) && is_digit(strings.substring(text, i, i + 1)) {
        n = n + 1
        i = i + 1
    }
    n
}
"#;

#[test]
fn substring_end_i_plus_one_must_not_emit_i32_into_usize() {
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
    let bad = generated.contains("+ 1_i32")
        || generated.contains("+ 1i32")
        || generated.contains("1_i32) as usize")
        || generated.contains("(i + 1_i32)");
    assert!(
        !bad,
        "RED P3.300: substring(text, i, i+1) must not emit i+1_i32 into usize cast:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (duration scanner shape):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
