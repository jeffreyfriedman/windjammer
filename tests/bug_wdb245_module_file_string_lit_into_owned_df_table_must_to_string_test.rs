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

//! WDB-245: `"props"` into `relational_df_sql_via_table_provider` must match formal ownership.
//!
//! Historically owned `left_table: String` required `"props".to_string()`. Tip now demotes
//! `left_table: &str` — bare `"props"` is correct. Gate accepts either owned+to_string or
//! demoted+bare.

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
    let mut demoted = false;
    for path in &ffi_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("ffi");
        if !text.contains("fn relational_df_sql_via_table_provider(") {
            continue;
        }
        if text.contains("left_table: String") {
            owned = true;
            break;
        }
        if text.contains("left_table: &str") {
            demoted = true;
            break;
        }
    }
    assert!(
        owned || demoted,
        "WDB-245: left_table formal missing (String or &str)"
    );

    // Prefer tip-out when present (gen may lag behind tip multipass).
    let paths = if tip.join("relational_df_analytic_execute_port.rs").exists() {
        vec![tip.join("relational_df_analytic_execute_port.rs")]
    } else {
        vec![gen.join("relational/relational_df_analytic_execute_port.rs")]
    };
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("analytic");
        let bare = text.contains("relational_df_sql_via_table_provider(left, \"props\",")
            || text.contains("relational_df_sql_via_table_provider(left,\"props\",");
        let has_to_string = text.contains("\"props\".to_string()")
            || text.contains("String::from(\"props\")");
        let bad = if owned {
            bare && !has_to_string
        } else {
            // Demoted &str: `.to_string()` / String::from into the call is the RED shape.
            text.contains(
                "relational_df_sql_via_table_provider(left, \"props\".to_string()",
            ) || text.contains(
                "relational_df_sql_via_table_provider(left, String::from(\"props\")",
            )
        };
        eprintln!(
            "WDB-245 owned={} demoted={} bare={} has_to_string={} bad={} path={}",
            owned,
            demoted,
            bare,
            has_to_string,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-245 RED: tip-out/product call-site ownership mismatches left_table formal. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-245: relational_df_analytic_execute_port missing");
}
