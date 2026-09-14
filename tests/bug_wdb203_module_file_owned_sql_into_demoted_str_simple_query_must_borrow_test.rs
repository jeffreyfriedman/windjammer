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

//! WDB-203: owned `String` sql into demoted `&str` `on_simple_query` must borrow.
//!
//! Product residual, gen unified_port:
//!   `pg_wire_serve_on_simple_query(state, sql)` while formal is `sql: &str`
//!   and local `sql` is owned `String` → expected `&str`, found `String`.
//! Inverse of WDB-196(B) / twin of WDB-181. Signature-driven.

use std::path::PathBuf;

#[test]
fn wdb203_product_unified_must_borrow_owned_sql_into_demoted_simple_query() {
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let unified = gen.join("relational/relational_pg_serve_unified_port.rs");
    let serve = gen.join("relational/relational_pg_serve_port.rs");
    if !unified.exists() || !serve.exists() {
        eprintln!("WDB-203: skip — unified/serve missing");
        return;
    }
    let unified_text = std::fs::read_to_string(&unified).expect("unified");
    let serve_text = std::fs::read_to_string(&serve).expect("serve");
    let demoted = serve_text
        .contains("fn pg_wire_serve_on_simple_query(state: PgWireServeState, sql: &str)")
        || serve_text.contains("sql: &str) -> (PgWireServeState");
    let bad = demoted
        && unified_text.contains("pg_wire_serve_on_simple_query(state, sql)")
        && !unified_text.contains("pg_wire_serve_on_simple_query(state, &sql)")
        && !unified_text.contains("pg_wire_serve_on_simple_query(state.clone(), &sql");
    if tip.join("relational_pg_serve_unified_port.rs").exists() {
        let tip_text =
            std::fs::read_to_string(tip.join("relational_pg_serve_unified_port.rs")).unwrap_or_default();
        eprintln!(
            "WDB-203 tip-out has_bare={}",
            tip_text.contains("pg_wire_serve_on_simple_query(state, sql)")
                && !tip_text.contains("&sql")
        );
    }
    eprintln!(
        "WDB-203 product demoted={} bad={} path={}",
        demoted,
        bad,
        unified.display()
    );
    assert!(
        !bad,
        "WDB-203 RED: product passes owned sql into demoted &str on_simple_query. {}",
        unified.display()
    );
}
