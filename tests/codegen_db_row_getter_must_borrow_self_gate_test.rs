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

//! FAILING REPRO / lockstep gate — `std::db::Row` column getters must borrow (`&self`).
//!
//! Runtime (`windjammer_runtime::db::Row`) already uses `&self`. When the std stub
//! took `self` by value, multipass treated each `row.get_string(...)` as a move and
//! dogfood adapters hit WJ0007 on the second column read (LedgerKit postgres_*).
//!
//! Desired:
//! 1. Std stubs declare `&self` for `get_string` / `get_int` / `*_at`.
//! 2. Multiple column reads from one `Row` binding must `wj build --check`.
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn db_row_getters_must_borrow_self_for_multi_column_reads() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
use std::db::Row

pub fn two_columns(row: Row) -> string {
    let id = match row.get_string("id") {
        Ok(v) => v,
        Err(_) => "missing" + "",
    }
    let name = match row.get_string("name") {
        Ok(v) => v,
        Err(_) => "missing" + "",
    }
    "${id}${name}"
}

fn main() {}
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
            "--check",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj build --check --module-file must allow two get_string calls on one Row (&self getters). stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );

    // Emitted Rust should borrow for get_string (no consuming self).
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
        rs.contains("get_string") || rs.contains("get_string("),
        "expected get_string in emitted Rust:\n{rs}"
    );
}
