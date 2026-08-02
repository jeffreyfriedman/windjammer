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
))]

//! FAILING REPRO — bare `""` in `Option` match arms opposite owned `string` must be owned.
//!
//! Tip emits `None => ""` as `&str` when `Some(s) => s` is `String` → E0308.
//! Desired: `None => String::new()` (same as bare `""` as a function return).
//! Forbidding `"".to_string()` means tip must get this right without that Rustism.
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn bare_empty_string_option_match_arm_must_emit_string_new() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
pub fn or_empty(opt: Option<string>) -> string {
    match opt {
        Some(s) => s,
        None => "",
    }
}

fn main() {
    let _ = or_empty(None)
}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj build must succeed. stderr=\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("t.rs")).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .find_map(|e| {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "rs") {
                    fs::read_to_string(p).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    assert!(
        !rs.contains("None => \"\""),
        "bare empty match arm must not stay as &str literal. Got:\n{rs}"
    );
    assert!(
        rs.contains("String::new()") || rs.contains("None => String::from(\"\")"),
        "expected owned empty in None arm. Got:\n{rs}"
    );

    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "empty_match_arm_gate"
version = "0.0.0"
edition = "2021"
[[bin]]
name = "empty_match_arm_gate"
path = "src/main.rs"
"#,
    )
    .unwrap();
    fs::write(
        crate_dir.join("src/main.rs"),
        format!("#![allow(dead_code, unused)]\n{rs}"),
    )
    .unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "bare `\"\"` Option match arm must cargo check without `\"\".to_string()`. stderr=\n{}\nemitted=\n{rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
