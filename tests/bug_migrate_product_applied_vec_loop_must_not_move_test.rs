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

//! P3.528: `apply_conn` loops `version_applied(applied, …)` with owned
//! `applied: Vec<int>` → rustc E0382 move in previous iteration.
//!
//! Isolates emit `applied.clone()` and cargo-check (false-GREEN).
//! `pending()` already emits `version_applied(&applied, …)`. Do not reshape
//! wj-migrate.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn migrate_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-migrate");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-migrate");
            if uncle.join("src").exists() {
                return Some(uncle);
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

fn moves_applied_in_loop(rs: &str) -> bool {
    rs.lines().any(|l| {
        let t = l.trim();
        t.contains("version_applied(applied,") && !t.contains("applied.clone()")
    })
}

#[test]
fn product_applied_vec_loop_must_not_move() {
    let pkg = migrate_pkg().unwrap_or_else(|| {
        panic!(
            "wj-migrate src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .current_dir(&pkg)
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
        .expect("wj build wj-migrate");
    assert!(
        build.status.success(),
        "wj-migrate product transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let apply_rs = fs::read_to_string(out.join("db_apply.rs")).unwrap_or_default();
    let hit = apply_rs
        .lines()
        .find(|l| l.contains("version_applied("))
        .unwrap_or("")
        .to_string();
    eprintln!("P3.528 apply_conn: {hit}");
    assert!(
        !moves_applied_in_loop(&apply_rs),
        "P3.528 RED: apply_conn must borrow or clone applied each loop:\n{hit}\n{apply_rs}"
    );
}
