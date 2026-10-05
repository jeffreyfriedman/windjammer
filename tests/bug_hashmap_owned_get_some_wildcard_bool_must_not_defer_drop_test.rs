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

//! P3.676: owned `HashMap` helper returning `bool` from `match map.get(key)`
//! with `Some(_)` must not emit `matches!(…)` then mid-body `DEFER DROP`
//! `thread::spawn(move || drop(map))` (E0308 / missing `;`).
//!
//! P3.267 greened mid-match defer-drop for full `match` forms; tip still
//! collapses `Some(_)` → `matches!` and splices defer-drop after it, making
//! the function return `()` from spawn. Product: `wj-dotenv::has`.
//! Do not reshape packages with `Some(v)` peels.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn hashmap_owned_get_some_wildcard_bool_must_not_defer_drop_after_matches() {
    let dir = TempDir::new().expect("tempdir");
    let src_dir = dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("lib.wj"),
        r#"
use std::collections::HashMap

pub fn has(map: HashMap<string, string>, key: string) -> bool {
    match map.get(key) {
        Some(_) => true,
        None => false,
    }
}
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("wj.toml"),
        r#"
[package]
name = "p3676-has"
version = "0.1.0"
"#,
    )
    .unwrap();

    let out = dir.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .current_dir(dir.path())
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.676 has emit:\n{rs}");

    let defer_after_matches = rs.contains("matches!(")
        && rs.contains("DEFER DROP")
        && rs.contains("thread::spawn");
    assert!(
        !defer_after_matches,
        "P3.676 RED: Some(_)→matches! must not splice DEFER DROP spawn after it:\n{rs}"
    );

    let status = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .status()
        .expect("cargo check");
    assert!(
        status.success(),
        "P3.676 RED: owned HashMap has(Some(_)) must cargo-check"
    );
}
