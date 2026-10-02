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

//! P3.568: `let n = strings.len(body)` must unify with `int` loop counters.
//!
//! Product `wj-auth-api` `tests/auth_test.wj` `json_string_field`:
//!   `let mut i = 0` / `while i + marker_len <= n` / `strings.substring(body, j, j + 1)`
//! Tip emits `i = 0_i64`, `n = strings::len(body)` (usize), then
//! `while i + (marker_len as i64) <= n` and `substring(..., j, …)` → E0308.
//! Distinct from P3.452 (substring arg wrap) / P3.524 (find_char &str).

use std::fs;
use std::process::Command;

#[test]
fn auth_json_string_field_int_len_must_unify_i64() {
    let src = tempfile::TempDir::new().expect("src");
    fs::write(
        src.path().join("lib.wj"),
        r#"use std::strings

pub fn json_string_field(body: string, key: string) -> string {
    let marker = "\"${key}\":\""
    let mut i = 0
    let n = strings.len(body)
    let marker_len = strings.len(marker)
    while i + marker_len <= n {
        if strings.substring(body, i, i + marker_len) == marker {
            let start = i + marker_len
            let mut out = ""
            let mut j = start
            while j < n {
                let ch = strings.substring(body, j, j + 1)
                if ch == "\"" {
                    return out
                }
                out = "${out}${ch}"
                j = j + 1
            }
            return ""
        }
        i = i + 1
    }
    ""
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
        "P3.568 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib_rs = fs::read_to_string(out.path().join("lib.rs")).unwrap_or_default();
    eprintln!("P3.568 lib.rs:\n{lib_rs}");

    let cargo = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("wj_p3568_cargo_{}", std::process::id())),
        )
        .output()
        .expect("cargo check");
    assert!(
        cargo.status.success(),
        "P3.568 RED: int loop + strings.len must cargo-check (auth json_string_field):\n{}\nlib.rs:\n{lib_rs}",
        String::from_utf8_lossy(&cargo.stderr)
    );
}
