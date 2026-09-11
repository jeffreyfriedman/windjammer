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

//! FAILING REPRO — after `get_string` borrows (P3.237 GREEN), owned `(Row, string)`
//! chain call sites must **move** `row` into the next `col_string(row, …)`, not
//! `col_string(row.clone(), …)`. LedgerKit still uses `"col" + ""` / clones while
//! tip clones at every link of `two_columns`.
//!
//! Distinct from `bug_db_row_get_string_no_row_clone_test` (getter / return clone).

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str = include_str!("fixtures/library_multipass/db_row_get_string_match_arms.wj");

fn assert_chain_moves_owned_row(rs: &str) {
    assert!(
        rs.contains("fn two_columns") || rs.contains("two_columns"),
        "fixture must emit two_columns; got:\n{rs}"
    );
    assert!(
        !rs.contains("col_string(row.clone()"),
        "RED: owned Row chain must move into col_string(row, …), not row.clone(). Got:\n{rs}"
    );
}

#[test]
fn db_row_col_chain_call_site_must_move_owned_row() {
    let rs = test_utils::compile_single(SOURCE);
    assert_chain_moves_owned_row(&rs);
}

#[test]
fn hexagonal_db_row_col_chain_call_site_must_move_owned_row() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod row_map\n");
    project.add_file("domain/row_map.wj", SOURCE);
    project.add_file("adapters/mod.wj", "pub mod postgres\n");
    project.add_file(
        "adapters/postgres.wj",
        r#"
use super::super::domain::row_map::two_columns
use std::db::Row

pub fn map_two(row: Row) -> string {
    two_columns(row)
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal row chain compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("row_map.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing row_map.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    assert_chain_moves_owned_row(map.get(&key).expect("row_map.rs"));
}
