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

//! P3.682: LedgerKit `json_cors_error(status: int, …)` with `status == 401`
//! must not emit `status: i64` vs `401_i32` (E0277). Product dogfood uses
//! `http_status_eq(status, code)` until tip greens bare lit compares.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn transpile(src: &str) -> String {
    let tmp = TempDir::new().expect("tempdir");
    let dir = tmp.path().join("src");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("lib.wj"), src).unwrap();
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            dir.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.682 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    fs::read_to_string(out.join("lib.rs")).unwrap_or_default()
}

fn splits_i64_status_vs_i32_lit(rs: &str) -> bool {
    let has_i64_status = rs.contains("status: i64") || rs.contains("status:i64");
    let has_i32_lit = rs.contains("401_i32")
        || rs.contains("403_i32")
        || rs.contains("404_i32")
        || rs.contains("500_i32");
    has_i64_status && has_i32_lit
}

#[test]
fn int_formal_eq_http_status_lit_must_unify_width() {
    let rs = transpile(
        r#"
pub fn json_cors_error(status: int, message: string) -> string {
    if status == 401 {
        return "unauthorized"
    }
    if status == 403 {
        return "forbidden"
    }
    if status == 404 {
        return "not_found"
    }
    if status == 500 {
        return "internal"
    }
    message
}
"#,
    );
    eprintln!("P3.682 emit:\n{rs}");
    assert!(
        !splits_i64_status_vs_i32_lit(&rs),
        "P3.682 RED: int formal == status lit must not split i64 vs i32:\n{rs}"
    );
    let check_dir = TempDir::new().expect("checkdir");
    let src = check_dir.path();
    fs::write(src.join("lib.rs"), &rs).unwrap();
    fs::write(
        src.join("Cargo.toml"),
        r#"[package]
name = "p3681"
version = "0.1.0"
edition = "2021"
[lib]
path = "lib.rs"
"#,
    )
    .unwrap();
    let check = Command::new("cargo")
        .current_dir(src)
        .args(["check", "--offline"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "P3.682 cargo-check failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
