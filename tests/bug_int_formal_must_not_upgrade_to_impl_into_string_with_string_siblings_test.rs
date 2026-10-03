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

//! P3.599: int formals must not become `impl Into<String>` just because sibling
//! string formals take Into (StatusChip / builder Into upgrade over-applied).
//!
//! Product `wj-todo-cli` after P3.597 for-in GREEN:
//!   `fn validate_max_len(field: string, value: string, max: int)`
//! emits `max: impl Into<String>` then `require_max_len(..., max.into())` with
//! crate `require_max_len(..., max: i64)` → E0277/E0308.
//! Distinct from P3.598 StatusChip builder Into (Self-returning text setters).

use std::fs;
use std::process::Command;

#[test]
fn int_formal_must_not_upgrade_to_impl_into_string_with_string_siblings() {
    let src = tempfile::TempDir::new().expect("src");
    fs::write(
        src.path().join("lib.wj"),
        r#"use std::strings

pub fn require_max_len(field: string, value: string, max: int) -> Result<string, string> {
    if strings.len(value) > max {
        return Err("too long")
    }
    Ok(value)
}

pub fn validate_max_len(field: string, value: string, max: int) -> Result<string, string> {
    require_max_len(field, value, max)
}

pub fn check_title(title: string, max_title_len: int) -> Result<string, string> {
    validate_max_len("title", title, max_title_len)
}
"#,
    )
    .unwrap();

    let out = tempfile::TempDir::new().expect("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.599 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib_rs = fs::read_to_string(out.path().join("lib.rs")).unwrap_or_default();
    eprintln!("P3.599 lib.rs:\n{lib_rs}");

    let validate_line = lib_rs
        .lines()
        .find(|l| l.contains("fn validate_max_len("))
        .unwrap_or("")
        .to_string();
    assert!(
        !validate_line.contains("max: impl Into<String>"),
        "P3.599 RED: int formal max must not become impl Into<String>:\n{validate_line}\n{lib_rs}"
    );
    assert!(
        validate_line.contains("max: i64") || validate_line.contains("max: int"),
        "P3.599 RED: validate_max_len max must stay i64:\n{validate_line}"
    );

    let cargo = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("wj_p3599_cargo_{}", std::process::id())),
        )
        .output()
        .expect("cargo check");
    assert!(
        cargo.status.success(),
        "P3.599 RED: int max with string sibling Into must cargo-check:\n{}\n{lib_rs}",
        String::from_utf8_lossy(&cargo.stderr)
    );
}
