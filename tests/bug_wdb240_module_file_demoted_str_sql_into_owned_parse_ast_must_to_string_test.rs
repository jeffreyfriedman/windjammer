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

//! WDB-240: demoted `&str` `sql` into owned `String` `relational_sql_parse_ast` must `.to_string()`.
//!
//! Product residual tip-out/gen relational_df_analytic_execute_port (~37× String←&str):
//!   `relational_sql_parse_ast(sql: String)` called with bare `sql` while caller formal is `&str`
//!   → E0308 expected `String`, found `&str`.
//! Signature-driven: `sql.to_string()` / `.to_owned()`. Twin of WDB-191 (pg wire) for PEG parse.

use std::path::PathBuf;

#[test]
fn wdb240_tip_out_df_analytic_must_to_string_demoted_sql_into_owned_parse_ast() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let peg_paths = [
        tip.join("relational_sql_peg_port.rs"),
        gen.join("relational/relational_sql_peg_port.rs"),
    ];
    let mut owned_formal = false;
    for path in &peg_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("peg");
        if text.contains("fn relational_sql_parse_ast(sql: String") {
            owned_formal = true;
            break;
        }
    }
    assert!(
        owned_formal,
        "WDB-240: owned String formal for relational_sql_parse_ast missing"
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
        let has_demoted_sql = text.contains("sql: &str");
        let bad = has_demoted_sql
            && text.contains("relational_sql_parse_ast(sql)")
            && !text.contains("relational_sql_parse_ast(sql.to_string())")
            && !text.contains("relational_sql_parse_ast(sql.to_owned())")
            && !text.contains("relational_sql_parse_ast((*sql).to_string())");
        eprintln!(
            "WDB-240 owned_formal={} has_demoted_sql={} bad={} path={}",
            owned_formal,
            has_demoted_sql,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-240 RED: tip-out/product passes &str sql into owned parse_ast without .to_string(). {}",
            path.display()
        );
    }
    assert!(saw, "WDB-240: relational_df_analytic_execute_port missing");
}
