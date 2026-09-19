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

//! WDB-196: tip-out feed_unified string lit / demoted `&str` into owned String.
//!
//! Product residual (~22× String←&str). After tip-out→gen sync of feed_unified:
//!   A) `pg_wire_encode_startup_user_database("wdb", "wdb")` while formals are
//!      owned `String` → E0308 (bare `&str` lits).
//!   B) demoted `sql: &str` into `pg_wire_serve_unified_on_simple_query(…, sql: String)`
//!      with bare `sql` → E0308.
//! Signature-driven. Distinct from WDB-191 (pg_wire_parse path).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod wire
pub mod feed
"#;

const WIRE: &str = r#"
/// Owns user/database (product pg_wire_encode_startup_user_database).
pub fn encode_startup(user: string, database: string) -> Vec<int> {
    let _ = user.len() + database.len()
    Vec::new()
}

/// Owns sql (product pg_wire_serve_unified_on_simple_query).
pub fn unified_query(sql: string) -> int {
    sql.len()
}
"#;

const FEED: &str = r#"
use crate::wire::encode_startup
use crate::wire::unified_query

/// Multi-use read demotes toward `&str` (product feed_unified sql).
pub fn sql_len(sql: string) -> int {
    sql.len()
}

pub fn feed_unified(sql: string) -> int {
    let _ = sql_len(sql)
    // Product A: encode_startup("wdb", "wdb") into owned String — must to_string
    let _bytes = encode_startup("wdb", "wdb")
    // Product B: unified_query(sql) while demoted — must to_string
    unified_query(sql)
}
"#;

fn wdb196_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("feed.wj", FEED);
    test
}

#[test]
fn wdb196_module_file_string_lit_and_demoted_str_into_owned_must_to_string() {
    let test = wdb196_fixture();
    let map = test
        .compile()
        .expect("WDB-196 multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let feed_rs = map.get("feed.rs").expect("feed.rs");

    eprintln!("WDB-196 wire.rs:\n{wire_rs}\nfeed.rs:\n{feed_rs}");

    let encode_owned = {
        let i = wire_rs.find("fn encode_startup").unwrap_or(0);
        let sl = &wire_rs[i..wire_rs.len().min(i + 140)];
        (sl.contains("user: String") || sl.contains("user:String"))
            && !(sl.contains("user: &str") || sl.contains("user:&str"))
    };
    let lit_bad = feed_rs.contains("encode_startup(\"wdb\", \"wdb\")")
        && !feed_rs.contains("encode_startup(\"wdb\".to_string()")
        && !feed_rs.contains("String::from(\"wdb\")");
    let caller_demoted = {
        let i = feed_rs.find("fn feed_unified").unwrap_or(0);
        let sl = &feed_rs[i..feed_rs.len().min(i + 100)];
        sl.contains("sql: &str") || sl.contains("sql:&str")
    };
    let sql_bad = feed_rs.contains("unified_query(sql)")
        && !feed_rs.contains("unified_query(sql.to_string()");

    if encode_owned && lit_bad {
        panic!(
            "WDB-196 RED(A): owned encode_startup received bare string lits. \
             Product: pg_wire_encode_startup_user_database(\"wdb\", \"wdb\"). Got:\n{feed_rs}"
        );
    }
    if encode_owned && caller_demoted && sql_bad {
        panic!(
            "WDB-196 RED(B): demoted &str into owned unified_query without to_string. Got:\n{feed_rs}"
        );
    }
}

#[test]
fn wdb196_tip_out_feed_unified_must_to_string_into_owned_string_formals() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let feed = if tip.join("relational_pg_serve_feed_unified_port.rs").exists() {
        tip.join("relational_pg_serve_feed_unified_port.rs")
    } else {
        gen.join("relational/relational_pg_serve_feed_unified_port.rs")
    };
    if !feed.exists() {
        eprintln!("WDB-196: skip — feed_unified missing");
        return;
    }
    let text = std::fs::read_to_string(&feed).expect("feed");
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let wire = tip.join("relational_pg_wire_port.rs");
    let wire_text = if wire.exists() {
        std::fs::read_to_string(&wire).unwrap_or_default()
    } else {
        String::new()
    };
    let encode_owned = wire_text.contains("fn pg_wire_encode_startup_user_database(user: String");
    let mut bad = Vec::new();
    // Tip demotes encode to `&str` — bare lits are correct; only flag when formal is owned.
    if encode_owned
        && text.contains("pg_wire_encode_startup_user_database(\"wdb\", \"wdb\")")
        && !text.contains("pg_wire_encode_startup_user_database(\"wdb\".to_string()")
        && !text.contains("String::from(\"wdb\")")
    {
        bad.push("A: encode_startup bare \"wdb\" lits into owned String".to_string());
    }
    if text.contains("fn pg_wire_serve_feed_unified_client_buffer(")
        && text.contains("sql: &str")
        && text.contains("pg_wire_serve_unified_on_simple_query(")
        && text.contains(", sql)")
        && !text.contains("sql.to_string()")
    {
        bad.push("B: demoted &str sql into owned unified_on_simple_query".to_string());
    }
    eprintln!(
        "WDB-196 tip-out bad={} path={}",
        bad.len(),
        feed.display()
    );
    assert!(
        bad.is_empty(),
        "WDB-196 RED: tip-out feed_unified String ownership:\n{}",
        bad.join("\n")
    );
}
