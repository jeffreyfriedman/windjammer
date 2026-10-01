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

//! P3.566: multipass `--module-file` with crate-root `lib.wj` must not auto-inject
//! `use super::lib::Type` into sibling modules.
//!
//! Product `wj-migrate` `db_status.wj` uses `pending` / `discover_migrations` that
//! return `Vec<Migration>` without an explicit `use crate::Migration`. Tip injects
//! `use super::lib::Migration` → rustc E0432 (`lib` is not a submodule; `lib.wj`
//! is the crate root). Explicit `use crate::Item` greened; inferred auto-import RED.
//! (P3.563/564 were tip-truth catch-ups; P3.565 was find_char AsRef.)

use std::fs;
use std::process::Command;

#[test]
fn module_file_crate_root_type_must_not_import_super_lib() {
    let src = tempfile::TempDir::new().expect("src");
    fs::write(
        src.path().join("lib.wj"),
        r#"pub struct Item {
    pub n: int,
}

pub fn pending(all: Vec<Item>) -> Vec<Item> {
    all
}
"#,
    )
    .unwrap();
    fs::write(
        src.path().join("status.wj"),
        r#"use crate::pending

pub fn report(items: Vec<Item>) -> int {
    let left = pending(items)
    let mut count = 0
    for _ in left {
        count = count + 1
    }
    count
}
"#,
    )
    .unwrap();

    let out = tempfile::TempDir::new().expect("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.566 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let status_rs = fs::read_to_string(out.path().join("status.rs")).unwrap_or_default();
    let lib_rs = fs::read_to_string(out.path().join("lib.rs")).unwrap_or_default();
    eprintln!("P3.566 status.rs:\n{status_rs}\nlib.rs head:\n{}", &lib_rs[..lib_rs.len().min(400)]);

    assert!(
        !status_rs.contains("super::lib::"),
        "P3.566 RED: crate-root types must not auto-import as super::lib::… \
         (product wj-migrate db_status → E0432). status.rs:\n{status_rs}"
    );

    // Prefer crate::Item (or bare via super::*) — not a fictional lib submodule.
    let cargo = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("wj_p3566_cargo_{}", std::process::id())),
        )
        .output()
        .expect("cargo check");
    assert!(
        cargo.status.success(),
        "P3.566 RED: multipass crate-root type import must cargo-check:\n{}\nstatus.rs:\n{status_rs}",
        String::from_utf8_lossy(&cargo.stderr)
    );
}
