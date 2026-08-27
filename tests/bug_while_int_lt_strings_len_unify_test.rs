//! `while end < strings.len(s)` with an `int` counter must unify (E0308 i64 vs usize).
//!
//! Annotated `let mut end: int = 0` keeps the binding as `i64`; codegen must cast
//! `strings.len` (usize) rather than mark the counter as usize. Full `wj build`
//! cargo-checks the package (transpile-only can false-green for untyped `0`).

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

#[test]
fn package_while_int_lt_strings_len_must_cargo_check() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("wj.toml"),
        r#"[package]
name = "strlen-loop-seed"
version = "0.1.0"
edition = "2025"

[lib]
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/lib.wj"),
        r#"
use std::strings

pub fn scan(text: string) -> int {
    let mut end: int = 0
    while end < strings.len(text) {
        end = end + 1
    }
    end
}
"#,
    )
    .unwrap();

    let status = Command::new(test_utils::wj_binary())
        .args(["build", "src"])
        .current_dir(root)
        .status()
        .expect("run wj build");

    let generated = fs::read_to_string(root.join("build/lib.rs")).unwrap_or_default();
    assert!(
        status.success(),
        "while end < strings.len(text) must cargo-check (int vs usize):\n{generated}"
    );
    assert!(
        generated.contains("as i64")
            && (generated.contains("strings::len") || generated.contains("strings.len")),
        "expected usize→i64 cast on strings.len side, got:\n{generated}"
    );
}
