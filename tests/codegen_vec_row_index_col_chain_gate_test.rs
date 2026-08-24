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

//! FAILING REPRO — `(Row, T)` chain on `rows[0]` must not move out of index twice.
//!
//! Dogfood (`postgres_bank_import_repository.wj`, `postgres_analytics_outbox.wj`):
//! ```ignore
//! let (_, id) = col_string(rows[0], "id" + "")
//! ```
//! Tip emits `col_string(rows[0], …)` and rustc E0507 moves `Row` from index.
//!
//! Expected: bind `let row = rows[0]` once, then chain helpers on `row`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn vec_row_index_col_chain_must_not_move_from_index_twice() {
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
        Some(v) => v + "",
        None => "" + "",
    }
    (row, value)
}

pub fn first_id(rows: Vec<Row>) -> string {
    if rows.len() == 0 {
        return "" + ""
    }
    let (_, id) = col_string(rows[0], "id" + "")
    id + ""
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
        "col_string(rows[0], …) must cargo-check. stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );
}
