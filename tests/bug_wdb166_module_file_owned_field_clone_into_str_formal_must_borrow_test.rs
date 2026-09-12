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

//! WDB-166: owned `string` field `.clone()` into demoted `&str` formal must borrow.
//!
//! Product (`relational_df_analytic_execute_port` after tip demotes FFI sql to `&str`):
//!   `relational_df_sql_via_table_provider(…, emit.sql.clone())`
//! → E0308 expected `&str`, found `String` (~168 of this class in cargo check).
//!
//! WDB-159 covers bare `sql.clone()` / sequential owned dispatch locals.
//! This gate covers **struct field** clones into demoted text formals
//! (`&emit.sql` or keep owned formal — signature-driven, no hardcoded names).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod ffi
pub mod exec
"#;

const FFI: &str = r#"
/// Read-only SQL consumer — tip demotes to `&str` (product relational_df_ffi_port).
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
    // Product analytic path clones fields into multi-use provider calls.
    let ok = sql_via_provider(emit.table.clone(), emit.sql.clone())
    ok
}
"#;

fn wdb166_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("ffi.wj", FFI);
    test.add_file("exec.wj", EXEC);
    test
}

#[test]
fn wdb166_module_file_owned_field_clone_into_str_formal_must_borrow() {
    let test = wdb166_fixture();
    let map = test
        .compile()
        .expect("WDB-166 multipass compile should succeed");
    let ffi_rs = map.get("ffi.rs").expect("ffi.rs");
    let exec_rs = map.get("exec.rs").expect("exec.rs");

    eprintln!("WDB-166 ffi.rs:\n{ffi_rs}\nexec.rs:\n{exec_rs}");

    let demoted = ffi_rs.contains("sql: &str") || ffi_rs.contains("sql:&str");
    let owned_clone_into_str = exec_rs.contains("emit.sql.clone()")
        || exec_rs.contains("sql_via_provider(emit.table.clone(), emit.sql.clone())");
    let borrows_field = exec_rs.contains("&emit.sql")
        || exec_rs.contains("sql_via_provider(&emit.table, &emit.sql)")
        || exec_rs.contains("&emit.table");

    // Product FFI demotes read-only sql to &str. Tip must either keep owned String
    // (also GREEN) or, when demoted, borrow field clones (`&emit.sql`) — never
    // bare `.clone()` into `&str` (product analytic E0308 storm).
    if demoted {
        assert!(
            !owned_clone_into_str || borrows_field,
            "WDB-166 RED: demoted &str formal still receives emit.sql.clone(). Got exec:\n{exec_rs}\nffi:\n{ffi_rs}"
        );
    } else {
        eprintln!("WDB-166: tip kept owned sql formal (acceptable); product gate still requires no clone-into-&str");
    }
}

#[test]
fn wdb166_product_analytic_must_not_clone_sql_into_demoted_str() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(
        "windjammerdb/crates/wdb-layers/gen/relational_module_file/relational_df_analytic_execute_port.rs",
    );
    if !path.exists() {
        eprintln!("WDB-166: skip product gate — {} missing", path.display());
        return;
    }
    let text = std::fs::read_to_string(&path).expect("read analytic");
    let ffi = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/relational_module_file/relational_df_ffi_port.rs");
    let ffi_text = std::fs::read_to_string(&ffi).unwrap_or_default();
    let demoted = ffi_text.contains("sql: &str");
    let bad = demoted && text.contains("emit.sql.clone()");
    eprintln!(
        "WDB-166 product analytic demoted_sql={} emit.sql.clone()={}",
        demoted,
        text.contains("emit.sql.clone()")
    );
    assert!(
        !bad,
        "WDB-166 RED: product analytic still passes emit.sql.clone() into demoted &str sql formal."
    );
}
