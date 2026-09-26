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

//! WDB-203: owned `String` sql into demoted `&str` `on_simple_query` must borrow.
//!
//! Product residual, gen unified_port:
//!   `pg_wire_serve_on_simple_query(state, sql)` while formal is `sql: &str`
//!   and local `sql` is owned `String` → expected `&str`, found `String`.
//! Inverse of WDB-196(B) / twin of WDB-181. Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod serve
pub mod unified
"#;

const SERVE: &str = r#"
/// Read-only SQL probe — tip demotes to `&str` (product on_simple_query).
pub fn on_simple_query(state: int, sql: string) -> int {
    state + sql.len()
}
"#;

const UNIFIED: &str = r#"
use crate::serve::on_simple_query

pub fn make_sql() -> string {
    "select 1"
}

pub fn serve(state: int) -> int {
    let sql = make_sql()
    // Product: owned String local into demoted &str — must &sql
    on_simple_query(state, sql)
}
"#;

fn wdb203_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("serve.wj", SERVE);
    test.add_file("unified.wj", UNIFIED);
    test
}

#[test]
fn wdb203_module_file_owned_sql_into_demoted_str_must_borrow() {
    let test = wdb203_fixture();
    let map = test
        .compile()
        .expect("WDB-203 multipass compile should succeed");
    let serve_rs = map.get("serve.rs").expect("serve.rs");
    let unified_rs = map.get("unified.rs").expect("unified.rs");
    eprintln!("WDB-203 isolate serve.rs:\n{serve_rs}\nunified.rs:\n{unified_rs}");
    let demoted = {
        let i = serve_rs.find("fn on_simple_query").unwrap_or(0);
        let sl = &serve_rs[i..serve_rs.len().min(i + 140)];
        sl.contains("sql: &str") || sl.contains("sql:&str")
    };
    let borrowed = unified_rs.contains("on_simple_query(state, &sql")
        || unified_rs.contains("on_simple_query(state,&sql");
    let bare = unified_rs.contains("on_simple_query(state, sql)")
        && !unified_rs.contains("on_simple_query(state, &sql");
    if demoted {
        assert!(
            borrowed && !bare,
            "WDB-203 RED: demoted &str received owned sql. Got:\n{unified_rs}\n{serve_rs}"
        );
    }
    test.cargo_check()
        .expect("WDB-203: owned sql into demoted &str must cargo-check");
}

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
