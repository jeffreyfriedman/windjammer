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

//! WDB-245: string lit into owned `String` `relational_df_sql_via_table_provider` must `.to_string()`.
//!
//! Product residual tip-out/gen relational_df_analytic_execute_port (~37× String←&str):
//!   `relational_df_sql_via_table_provider(..., left_table: String, …)`
//!   called with bare `"props"` → E0308 expected `String`, found `&str`.
//! Twin of WDB-221 (lsqb edge_kind lit). Signature-driven: `"props".to_string()`.

use std::path::PathBuf;

#[test]
fn wdb245_tip_out_df_analytic_must_to_string_props_lit_into_owned_table_provider() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let ffi_paths = [
        tip.join("relational_df_ffi_port.rs"),
        gen.join("relational/relational_df_ffi_port.rs"),
    ];
    let mut owned = false;
    for path in &ffi_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("ffi");
        if text.contains("fn relational_df_sql_via_table_provider(")
            && text.contains("left_table: String")
        {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-245: owned String left_table formal for table_provider missing"
    );

    let paths = [
        tip.join("relational_df_analytic_execute_port.rs"),
        gen.join("relational/relational_df_analytic_execute_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("analytic");
        let bare = text.contains("relational_df_sql_via_table_provider(left, \"props\",")
            || text.contains("relational_df_sql_via_table_provider(left,\"props\",");
        let has_to_string = text.contains("\"props\".to_string()");
        let bad = bare && !has_to_string;
        eprintln!(
            "WDB-245 owned={} bare={} has_to_string={} bad={} path={}",
            owned,
            bare,
            has_to_string,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-245 RED: tip-out/product passes bare \"props\" lit into owned String table formal. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-245: relational_df_analytic_execute_port missing");
}
