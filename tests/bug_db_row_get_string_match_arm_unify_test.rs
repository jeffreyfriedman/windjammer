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

//! FAILING REPRO — `match row.get_string(col) { Ok(v) => v, Err(_) => "" }` must
//! unify to owned `String` (LedgerKit drops `v + ""` / `"" + ""` workarounds).

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str = include_str!("fixtures/library_multipass/db_row_get_string_match_arms.wj");

#[test]
fn db_row_get_string_match_arms_must_unify_owned_string() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert!(
        ok,
        "RED: get_string Ok(v)/Err(\"\") arms must unify to owned String without +\"\". Generated:\n{rs}"
    );
    // Prefer owning the empty arm (or borrowing both), not mixing String with &str.
    let empty_arm_bare = rs.contains("Err(_) => \"\"") || rs.contains("Err(_) => \"\",");
    let empty_arm_owned = rs.contains("Err(_) => String::")
        || rs.contains("Err(_) => \"\".to_string()")
        || rs.contains("Err(_) => String::new()");
    assert!(
        empty_arm_owned || !empty_arm_bare,
        "RED: Err empty-literal arm must own (or both arms borrow). Got:\n{rs}"
    );
}

#[test]
fn hexagonal_db_row_get_string_match_arms_must_cargo_check() {
    // Emit gate only here: `std::db` pulls sqlx/`libsqlite3-sys`, whose build script
    // fails under agent sandboxes (PermissionDenied copying bindings). The owned-arm
    // unify is covered by `db_row_get_string_match_arms_must_unify_owned_string`;
    // set WJ_ENABLE_DB_CARGO_CHECK=1 to also run multipass cargo-check locally.
    let mut project = MultiFileTest::new().with_runtime_features(&["db"]);
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
        .expect("hexagonal get_string match compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("row_map.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing row_map.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    let rs = map.get(&key).expect("row_map.rs");
    assert!(
        rs.contains("get_string"),
        "must call get_string; got:\n{rs}"
    );
    let empty_arm_owned = rs.contains("Err(_) => String::")
        || rs.contains("Err(_) => \"\".to_string()")
        || rs.contains("Err(_) => String::new()");
    assert!(
        empty_arm_owned,
        "RED: hexagonal Err empty-literal arm must own. Got:\n{rs}"
    );

    if std::env::var_os("WJ_ENABLE_DB_CARGO_CHECK").is_some() {
        project
            .cargo_check()
            .expect("hexagonal get_string match arms without +\"\" must cargo-check");
    }
}
