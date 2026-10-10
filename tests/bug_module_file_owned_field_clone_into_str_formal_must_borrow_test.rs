#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Regression: module file owned field clone into str formal must borrow.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod ffi
pub mod exec
"#;

const FFI: &str = r#"
/// Read-only SQL consumer — codegen demotes to `&str`.
pub fn sql_via_provider(table: string, sql: string) -> bool {
    table.len() > 0 && sql.len() > 0
}
"#;

const EXEC: &str = r#"
use crate::ffi::sql_via_provider

pub struct Emit {
    pub table: string,
    pub sql: string,
}

pub fn run_emit(emit: Emit) -> bool {
    sql_via_provider(emit.table, emit.sql)
}

pub fn run_emit_clone_path(emit: Emit) -> bool {
    // Analytic path clones fields into multi-use provider calls.
    let ok = sql_via_provider(emit.table.clone(), emit.sql.clone())
    ok
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("ffi.wj", FFI);
    test.add_file("exec.wj", EXEC);
    test
}

#[test]
fn module_file_owned_field_clone_into_str_formal_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let ffi_rs = map.get("ffi.rs").expect("ffi.rs");
    let exec_rs = map.get("exec.rs").expect("exec.rs");

    eprintln!("ffi.rs:\n{ffi_rs}\nexec.rs:\n{exec_rs}");

    let demoted = ffi_rs.contains("sql: &str") || ffi_rs.contains("sql:&str");
    let owned_clone_into_str = exec_rs.contains("emit.sql.clone()")
        || exec_rs.contains("sql_via_provider(emit.table.clone(), emit.sql.clone())");
    let borrows_field = exec_rs.contains("&emit.sql")
        || exec_rs.contains("sql_via_provider(&emit.table, &emit.sql)")
        || exec_rs.contains("&emit.table");

    // FFI demotes read-only sql to &str. Codegen must either keep owned String
    // (also GREEN) or, when demoted, borrow field clones (`&emit.sql`) — never
    // bare `.clone()` into `&str`.
    if demoted {
        assert!(
            !owned_clone_into_str || borrows_field,
            "RED: demoted &str formal still receives emit.sql.clone(). Got exec:\n{exec_rs}\nffi:\n{ffi_rs}"
        );
    } else {
        eprintln!("kept owned sql formal (acceptable)");
    }
}
