#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Multi-file CLI with `src/main.wj` + domain modules must produce a runnable binary:
//! - `main.rs` keeps `fn main` (not stripped / not skipped as a library module)
//! - `Cargo.toml` has both `[lib]` and `[[bin]]`
//! - binary can `use` domain via the package lib name
//!
//! Bug (ecosystem wj-hello): building `src/` emitted lib-only Cargo.toml and a main.rs
//! with `use super::*` and no `fn main`, so `cargo run` failed with E0601 / no bin target.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multi_file_cli_emits_lib_and_bin_with_main() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let domain = src.join("domain");
    let build = temp.path().join("build");
    fs::create_dir_all(&domain).unwrap();

    fs::write(
        temp.path().join("wj.toml"),
        r#"[package]
name = "wj-hello"
version = "0.1.0"
edition = "2025"
"#,
    )
    .unwrap();

    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(domain.join("mod.wj"), "pub mod version\npub use version::version\n").unwrap();
    fs::write(
        domain.join("version.wj"),
        "pub fn version() -> string {\n    \"wj-hello 0.1.0\"\n}\n",
    )
    .unwrap();
    fs::write(
        src.join("main.wj"),
        r#"use crate::domain::version

fn main() {
    println!("{}", version())
}
"#,
    )
    .unwrap();

    let wj = test_utils::wj_binary();
    let output = Command::new(&wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            build.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(temp.path())
        .output()
        .expect("run wj build");

    assert!(
        output.status.success(),
        "wj build failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let main_rs = fs::read_to_string(build.join("main.rs")).expect("main.rs");
    assert!(
        main_rs.contains("fn main("),
        "main.rs must keep fn main for CLI entry:\n{main_rs}"
    );
    assert!(
        !main_rs.contains("use super::*;"),
        "crate-root main.rs must not inject use super::*:\n{main_rs}"
    );

    let cargo = fs::read_to_string(build.join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        cargo.contains("[lib]"),
        "expected [lib] for domain modules:\n{cargo}"
    );
    assert!(
        cargo.contains("[[bin]]"),
        "expected [[bin]] for main.rs:\n{cargo}"
    );
    assert!(
        cargo.contains("path = \"main.rs\""),
        "[[bin]] must point at main.rs:\n{cargo}"
    );
}
