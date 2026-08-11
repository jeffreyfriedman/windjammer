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

//! FAILING REPRO — `strings::starts_with(s, "lit")` must keep Pattern/`&str` borrow.
//!
//! Runtime `starts_with` takes `prefix: &str`. Tip must not emit
//! `starts_with(..., "Bearer ".to_string())` (E0308). Same class as
//! `trim_end_matches("/")` and `env::get("KEY")`.
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn strings_starts_with_literal_must_not_auto_own() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
use std::strings

pub fn is_bearer(header: string) -> bool {
    strings.starts_with(header, "Bearer ")
}

fn main() {
    let _ = is_bearer("Bearer x")
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
        !rs.contains("\"Bearer \".to_string()")
            && !rs.contains("\"Bearer \".to_owned()"),
        "must not auto-own prefix literal into starts_with(&str). Got:\n{rs}"
    );

    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime");
    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "starts_with_str_literal_gate"
version = "0.0.0"
edition = "2021"
[dependencies]
windjammer-runtime = {{ path = "{}" }}
[[bin]]
name = "starts_with_str_literal_gate"
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
        "starts_with with bare string literal must cargo check. stderr=\n{}\nemitted=\n{rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
