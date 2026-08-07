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

//! FAILING REPRO — `int` formals must not demote to `&i64`.
//!
//! Read-only / by-value integer parameters in library multipass emit must stay
//! `i64` (or `i32`), never `&i64`, so call sites can pass literals (`fn(2, 1)`).
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn wj_library(src_mod: &std::path::Path, out: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src_mod.to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run wj")
}

#[test]
fn int_formals_must_not_demote_to_ref_i64() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();
    fs::write(
        src.join("counts.wj"),
        r#"
pub fn status_html(class_count: int, dept_count: int) -> string {
    "c=${class_count},d=${dept_count}"
}
"#,
    )
    .unwrap();
    fs::write(src.join("mod.wj"), "pub mod counts\n").unwrap();

    let output = wj_library(&src.join("mod.wj"), &out);
    assert!(
        output.status.success(),
        "wj library build must succeed. stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(out.join("counts.rs")).expect("counts.rs");
    let generated = generated
        .replace("#[allow(unused_imports)]\nuse super::*;\n\n", "")
        .replace("#[allow(unused_imports)]\nuse super::*;\n", "");
    assert!(
        !generated.contains("class_count: &i64")
            && !generated.contains("dept_count: &i64")
            && !generated.contains(": &i64,"),
        "int formals must stay by-value i64, not &i64. Got:\n{generated}"
    );
    assert!(
        generated.contains("class_count: i64") || generated.contains("class_count: i32"),
        "expected owned int formal. Got:\n{generated}"
    );

    fs::write(
        out.join("Cargo.toml"),
        r#"[package]
name = "int_formal_gate"
version = "0.0.0"
edition = "2021"
[workspace]
[lib]
path = "lib.rs"
"#,
    )
    .unwrap();
    fs::write(
        out.join("lib.rs"),
        format!("{generated}\nfn smoke() -> String {{ status_html(2, 1) }}\n"),
    )
    .unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&out)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "literal int call sites must cargo check. stderr=\n{}\nemitted=\n{generated}",
        String::from_utf8_lossy(&check.stderr)
    );
}
