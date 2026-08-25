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

//! FAILING REPRO — comparing a demoted `&str` local to a string literal must not emit
//! `local.as_str()` (unstable `str_as_str` on older rustc → E0658).
//!
//! Dogfood (finance-screens tip): `let fmt = format_query.trim(); if fmt == "csv"` emits
//! `fmt.as_str() == "csv"`. Product workaround: `"${fmt}" == "csv"` (stable String.as_str)
//! or `(fmt + "") == "csv"`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn trim_local_eq_literal_must_not_emit_as_str() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
pub fn is_csv(format_query: string) -> bool {
    let fmt = format_query.trim()
    fmt == "csv"
}

fn main() {
    let _ = is_csv("csv")
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
        !rs.contains(".as_str()"),
        "trim local vs literal must not emit .as_str(). Got:\n{rs}"
    );

    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "trim_eq_no_as_str_gate"
version = "0.0.0"
edition = "2021"
[[bin]]
name = "trim_eq_no_as_str_gate"
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
        "trim eq literal must cargo check without str_as_str. stderr=\n{}\nemitted=\n{rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
