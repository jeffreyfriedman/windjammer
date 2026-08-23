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

//! FAILING REPRO / lockstep gate — chain `(Row, T)` column reads without `&Row`.
//!
//! Dogfood postgres adapters must read multiple columns from one `Row` without:
//! - explicit `&Row` params (W0001 rust-leakage lint), or
//! - by-value helper params that move `Row` on each call (WJ0007).
//!
//! Desired: helpers returning `(Row, T)` chain under `wj build --check --module-file`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn db_row_col_string_chain_must_compile_for_multi_column_reads() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
use std::db::Row

fn col_string(row: Row, col: string) -> (Row, string) {
    let value = match row.get_string(col + "") {
        Ok(v) => v + "",
        Err(_) => "" + "",
    }
    (row, value)
}

pub fn two_columns(row: Row) -> string {
    let (row, id) = col_string(row, "id" + "")
    let (_row, name) = col_string(row, "name" + "")
    "${id}:${name}"
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
        "wj build --check must allow (Row, T) chaining for multi-column Row reads. stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );
}
