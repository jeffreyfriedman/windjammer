//! Package-shaped `std::csv.write(owned rows)` must auto-borrow to `&[Vec<String>]`.
//!
//! When the wrapper is named `serialize`, codegen correctly emits `csv::write(&rows)`.
//! Ecosystem `wj-csv` uses idiomatic `pub fn write(...)` forwarding to `csv.write(rows)`,
//! which emits `csv::write(rows.clone())` without `&` → E0308.
//! Keep pure WJ `write` until this gate is green.

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

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

fn build_package_src(src: &str) -> String {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("wj.toml"),
        r#"[package]
name = "csv-write-seed"
version = "0.1.0"
edition = "2025"

[lib]
"#,
    )
    .unwrap();
    fs::write(root.join("src/lib.wj"), src).unwrap();

    let status = Command::new(test_utils::wj_binary())
        .args(["build", "--no-cargo", "src"])
        .current_dir(root)
        .status()
        .expect("run wj build");
    assert!(status.success(), "wj build --no-cargo src failed");
    fs::read_to_string(root.join("build/lib.rs")).expect("read build/lib.rs")
}

#[test]
fn package_csv_write_homonym_owned_rows_must_auto_borrow() {
    let generated = build_package_src(
        r#"
use std::csv

pub fn write(rows: Vec<Vec<string>>) -> Result<string, string> {
    csv.write(rows)
}
"#,
    );

    assert!(
        generated.contains("csv::write"),
        "expected qualified csv::write call site:\n{generated}"
    );
    let bare_owned = generated.contains("write(rows.clone())")
        && !generated.contains("write(&rows.clone())");
    let borrowed = generated.contains("write(&rows)")
        || generated.contains("write(&rows.clone())")
        || generated.contains("write(rows.as_slice())");
    assert!(
        borrowed && !bare_owned,
        "user fn write forwarding to csv.write must borrow owned rows:\n{generated}"
    );
}
