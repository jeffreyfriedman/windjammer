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

//! FAILING REPRO — bare string literal into runtime `env::get(&str)` must NOT auto-own.
//!
//! `windjammer_runtime::env::get` takes `key: &str`. Tip Phase-5 typed_lowering must
//! not emit `env::get("KEY".to_string())` (E0308 expected `&str`, found `String`).
//! Opposite of owned-`String` builder formals: cross-crate `&str` formals keep borrows.
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn env_get_string_literal_must_not_auto_own_for_str_formal() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
use std::env

pub fn flag_set() -> bool {
    match env::get("DEMO_FLAG") {
        Some(_) => true,
        None => false,
    }
}

fn main() {
    let _ = flag_set()
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
        !rs.contains("env::get(\"DEMO_FLAG\".to_string())")
            && !rs.contains("env::get(\"DEMO_FLAG\".to_owned())")
            && !rs.contains("get(\"DEMO_FLAG\".to_string())"),
        "must not auto-own string literal into env::get(&str). Got:\n{rs}"
    );

    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime");
    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "env_get_str_literal_gate"
version = "0.0.0"
edition = "2021"
[dependencies]
windjammer-runtime = {{ path = "{}" }}
[[bin]]
name = "env_get_str_literal_gate"
path = "src/main.rs"
"#,
            runtime.display()
        ),
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
        "env::get with bare string literal must cargo check. stderr=\n{}\nemitted=\n{rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
