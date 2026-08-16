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

//! FAILING REPRO — nested `find` on locals (`after` / `inner`) must not emit
//! `compile_error!("missing boundary signature for …::find")`.
//!
//! Pattern: `json.find` → `substring` into `after` → `after.find` → `inner.find`.
//! Tip currently injects a crate-level `compile_error!` for those method boundaries
//! even when the emitted Rust would otherwise typecheck.
//!
//! Language-only; no product/repo names.
//! Blocks `--module-file` dogfood when JSON field helpers nest `find` on owned slices.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn nested_find_on_substring_locals_must_not_emit_boundary_compile_error() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
pub fn field_after_colon(json: string, key: string) -> string {
    let needle = "\"" + key + "\""
    match json.find(needle) {
        Some(idx) => {
            let after = json.substring(idx, json.len())
            match after.find(":") {
                Some(c) => {
                    let inner = after.substring(c + 1, after.len())
                    match inner.find("\"") {
                        Some(q) => inner.substring(0, q),
                        None => inner,
                    }
                },
                None => "",
            }
        },
        None => "",
    }
}

fn main() {
    let _ = field_after_colon("{\"a\":\"b\"}", "a")
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
        !rs.is_empty(),
        "expected generated Rust under {}",
        out.display()
    );
    assert!(
        !rs.contains("compile_error!") && !rs.contains("missing boundary signature"),
        "nested find on substring locals must not emit boundary compile_error. Got:\n{rs}"
    );

    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "nested_find_boundary_gate"
version = "0.0.0"
edition = "2021"
[[bin]]
name = "nested_find_boundary_gate"
path = "src/main.rs"
"#,
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
        "nested find+substring must cargo check. stderr=\n{}\nemitted=\n{rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
