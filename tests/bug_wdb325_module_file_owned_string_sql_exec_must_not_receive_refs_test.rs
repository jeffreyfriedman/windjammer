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
    feature = "codegen_tests",
))]

//! WDB-325: owned `string` sql_exec formals must not receive `&emit.table` / `&emit.sql`.
//!
//! Product tip-out/gen relational_df_analytic_execute_port:
//!   `left.sql_exec(right, &emit.table, &right_name, &emit.sql)`
//!   with `left_table: string, right_table: string, sql: string` → E0308
//!   (cross-crate `ArrowColumnarBatch::sql_exec` in wdb-types).
//! Twin of WDB-306/320 (owned String call args). `.len()` demotes to `&str` — force
//! owned via struct store (same pattern as WDB-320/322).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Emit {
    pub table: string,
    pub sql: string,
}

pub struct Host {
    pub ok: bool,
}

pub fn sql_exec(host: Host, left_table: string, right_table: string, sql: string) -> Emit {
    Emit {
        table: left_table + "_" + right_table,
        sql: sql,
    }
}

pub fn run(host: Host, emit: Emit, right_name: string) -> Emit {
    let _ = host.ok
    sql_exec(host, emit.table, right_name, emit.sql)
}
"#;

#[test]
fn wdb325_module_file_owned_string_sql_exec_must_not_receive_refs() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-325 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-325 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn sql_exec(host: Host, left_table: String")
        || rs.contains("fn sql_exec(host: Host, mut left_table: String");
    assert!(
        owned,
        "WDB-325: expected owned String formals on sql_exec:\n{rs}"
    );
    let bad = rs.contains("&emit.table")
        || rs.contains("&emit.sql")
        || rs.contains("&right_name")
        || rs.contains("sql_exec(host, &");
    assert!(
        !bad,
        "WDB-325 RED: owned sql_exec received &emit.table/&emit.sql/&right_name:\n{rs}"
    );
    test.cargo_check().expect("WDB-325 cargo-check");
}

#[test]
fn wdb325_tip_out_df_analytic_must_not_pass_ref_into_owned_sql_exec() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("relational_df_analytic_execute_port.rs"),
        tip.join("relational/relational_df_analytic_execute_port.rs"),
        gen.join("relational/relational_df_analytic_execute_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("df_analytic");
        let bad = text.lines().any(|line| {
            line.contains("sql_exec(")
                && (line.contains("&emit.table")
                    || line.contains("&emit.sql")
                    || line.contains("&right_name"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-325: tip-out/gen relational_df_analytic_execute_port missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-325 RED: tip-out/product passes &emit.*/&right_name into owned sql_exec in:\n  {}",
        bad_paths.join("\n  ")
    );
}
