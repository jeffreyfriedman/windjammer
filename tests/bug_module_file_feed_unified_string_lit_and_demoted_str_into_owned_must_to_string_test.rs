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
//!   A) `pg_wire_encode_startup_user_database("user", "user")` while formals are
//!      owned `String` → E0308 (bare `&str` lits).
//!   B) demoted `sql: &str` into `pg_wire_serve_unified_on_simple_query(…, sql: String)`
//!      with bare `sql` → E0308.
//! Signature-driven. Distinct from WDB-191 (pg_wire_parse path).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
    // Product A: encode_startup("user", "user") into owned String — must to_string
    let _bytes = encode_startup("user", "user")
    // Product B: unified_query(sql) while demoted — must to_string
    unified_query(sql)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("feed.wj", FEED);
    test
}

#[test]
fn module_file_string_lit_and_demoted_str_into_owned_must_to_string() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let feed_rs = map.get("feed.rs").expect("feed.rs");

    eprintln!("wire.rs:\n{wire_rs}\nfeed.rs:\n{feed_rs}");

    let encode_owned = {
        let i = wire_rs.find("fn encode_startup").unwrap_or(0);
        let sl = &wire_rs[i..wire_rs.len().min(i + 140)];
        (sl.contains("user: String") || sl.contains("user:String"))
            && !(sl.contains("user: &str") || sl.contains("user:&str"))
    };
    let lit_bad = feed_rs.contains("encode_startup(\"user\", \"user\")")
        && !feed_rs.contains("encode_startup(\"user\".to_string()")
        && !feed_rs.contains("String::from(\"user\")");
    let caller_demoted = {
        let i = feed_rs.find("fn feed_unified").unwrap_or(0);
        let sl = &feed_rs[i..feed_rs.len().min(i + 100)];
        sl.contains("sql: &str") || sl.contains("sql:&str")
    };
    let sql_bad = feed_rs.contains("unified_query(sql)")
        && !feed_rs.contains("unified_query(sql.to_string()");

    if encode_owned && lit_bad {
        panic!(
            "(A): owned encode_startup received bare string lits. \
             Product: pg_wire_encode_startup_user_database(\"user\", \"user\"). Got:\n{feed_rs}"
        );
    }
    if encode_owned && caller_demoted && sql_bad {
        panic!(
            "(B): demoted &str into owned unified_query without to_string. Got:\n{feed_rs}"
        );
    }
}
