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

//! FAILING REPRO — `trim_end_matches("/")` must emit a Pattern-compatible borrow.
//!
//! `str::trim_end_matches` requires `Pattern`; owned `String` is not a Pattern.
//! Tip must emit `trim_end_matches("/")` (or `&"/"`), not `"/".to_string()`.
//! Same class as `find(":".to_string())` — Pattern args must stay `&str`.
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn trim_end_matches_string_literal_must_borrow_for_pattern() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
pub fn strip_trailing_slash(base: string) -> string {
    base.trim_end_matches("/").to_string()
}

fn main() {
    let _ = strip_trailing_slash("https://example.com/")
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
        !rs.contains("trim_end_matches(\"/\".to_string())")
            && !rs.contains("trim_end_matches(\"/\".to_owned())"),
        "must not pass owned String into trim_end_matches (Pattern). Got:\n{rs}"
    );

    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "trim_end_matches_pattern_gate"
version = "0.0.0"
edition = "2021"
[[bin]]
name = "trim_end_matches_pattern_gate"
path = "src/main.rs"
"#,
    )
    .unwrap();
    fs::write(
        crate_dir.join("src/main.rs"),
        format!("#![allow(dead_code)]\n{rs}"),
    )
    .unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "trim_end_matches with string pattern must cargo check. stderr=\n{}\nemitted=\n{rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
