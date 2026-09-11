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

//! FAILING REPRO — `row.get_string(col)` must borrow `row` (or take `&Row`), not
//! emit `row.clone().get_string(...)`, so owned `(Row, string)` helpers can return
//! `row` by move. LedgerKit still chains via clones / `"col" + ""` workarounds
//! when tip clones for every get_string + tuple return.
//!
//! Distinct from `bug_db_row_get_string_match_arm_unify_test` (Ok/Err arm unify).

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str = include_str!("fixtures/library_multipass/db_row_get_string_match_arms.wj");

fn assert_no_row_clone_around_get_string(rs: &str) {
    // Ignore doc comments that may mention the anti-pattern by name.
    let code = rs
        .lines()
        .filter(|l| !l.trim_start().starts_with("///") && !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !code.contains("row.clone().get_string"),
        "RED: get_string must not force row.clone() before the call. Got:\n{rs}"
    );
    // Returning owned Row after a read must move `row`, not clone it again.
    let returns_cloned_row = code.contains("(row.clone(), value)")
        || code.contains("(row.clone(),value)")
        || code.contains("(row.clone(), id)")
        || code.contains("(row.clone(), name)");
    assert!(
        !returns_cloned_row,
        "RED: (Row, string) return must move row, not row.clone(). Got:\n{rs}"
    );
}

#[test]
fn db_row_get_string_must_not_emit_row_clone() {
    let rs = test_utils::compile_single(SOURCE);
    assert!(
        rs.contains("get_string"),
        "must emit get_string; got:\n{rs}"
    );
    assert_no_row_clone_around_get_string(&rs);
}

#[test]
fn hexagonal_db_row_get_string_must_not_emit_row_clone() {
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
        .expect("hexagonal get_string compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("row_map.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing row_map.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    let rs = map.get(&key).expect("row_map.rs");
    assert_no_row_clone_around_get_string(rs);
}
