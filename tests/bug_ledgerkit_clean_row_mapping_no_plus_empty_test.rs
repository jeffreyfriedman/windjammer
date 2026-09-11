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

//! REGRESSION / dogfood gate — LedgerKit-shaped `account_from_row` without
//! `+ ""` workarounds must emit clean get_string / chain moves (P3.241).
//! Tip probe 2026-09-10: CLEAN_CHECK:OK.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/ledgerkit_clean_account_from_row.wj");

fn assert_clean_ledgerkit_emit(rs: &str) {
    assert!(
        rs.contains("get_string"),
        "must call get_string; got:\n{rs}"
    );
    assert!(
        !rs.contains("row.clone().get_string"),
        "get_string must borrow row; got:\n{rs}"
    );
    assert!(
        !rs.contains("col_string(row.clone()"),
        "chain must move row; got:\n{rs}"
    );
    // No concat workarounds for empty / identity own.
    assert!(
        !rs.contains("format!(\"{}{}\", v, \"\")")
            && !rs.contains("format!(\"{}{}\", \"\", \"\")"),
        "must not emit +\"\" concat workarounds; got:\n{rs}"
    );
    let empty_arm_owned = rs.contains("Err(_) => String::new()")
        || rs.contains("Err(_) => String::from")
        || rs.contains("Err(_) => \"\".to_string()");
    let empty_arm_bare = rs.contains("Err(_) => \"\"");
    assert!(
        empty_arm_owned || !empty_arm_bare,
        "Err empty arm must own (or both arms borrow); got:\n{rs}"
    );
}

#[test]
fn ledgerkit_clean_account_from_row_must_emit_without_plus_empty() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert!(
        ok,
        "RED: clean LedgerKit account_from_row must transpile. Generated:\n{rs}"
    );
    assert_clean_ledgerkit_emit(&rs);
}

#[test]
fn hexagonal_ledgerkit_clean_account_from_row_must_emit_without_plus_empty() {
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
use super::super::domain::row_map::account_from_row
use std::db::Row

pub fn map_account(row: Row) -> bool {
    match account_from_row(row) {
        Some(_) => true,
        None => false,
    }
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal clean LedgerKit mapping compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("row_map.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing row_map.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    assert_clean_ledgerkit_emit(map.get(&key).expect("row_map.rs"));
}
