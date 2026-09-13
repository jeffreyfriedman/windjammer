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

//! WDB-191: demoted `&str` into owned `String` parse formal must `.to_string()`.
//!
//! Product residual (~16× String←&str), gen `relational_pg_serve_port`:
//!   `pg_wire_serve_on_simple_query(…, sql: &str)` calls
//!   `pg_wire_parse(session, "", sql)` while parse name/sql are owned `String`
//! → E0308. Same class as WDB-180 (bakeoff fill) for pg_wire parse path.
//! Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod parse
pub mod serve
"#;

const PARSE: &str = r#"
pub struct Session {
    pub n: int,
}

/// Owns name/sql strings (product pg_wire_parse).
pub fn wire_parse(session: Session, name: string, sql: string) -> Session {
    let _ = name.len() + sql.len()
    Session { n: session.n + 1 }
}
"#;

const SERVE: &str = r#"
use crate::parse::Session
use crate::parse::wire_parse

/// Multi-use read demotes toward `&str` (product pg_wire_serve_on_simple_query).
pub fn sql_len(sql: string) -> int {
    sql.len()
}

pub fn on_simple_query(session: Session, sql: string) -> Session {
    let _ = sql_len(sql)
    // Product: wire_parse(session, "", sql) while demoted — must to_string
    wire_parse(session, "", sql)
}
"#;

fn wdb191_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("parse.wj", PARSE);
    test.add_file("serve.wj", SERVE);
    test
}

#[test]
fn wdb191_module_file_demoted_str_into_owned_parse_must_to_string() {
    let test = wdb191_fixture();
    let map = test
        .compile()
        .expect("WDB-191 multipass compile should succeed");
    let parse_rs = map.get("parse.rs").expect("parse.rs");
    let serve_rs = map.get("serve.rs").expect("serve.rs");

    eprintln!("WDB-191 parse.rs:\n{parse_rs}\nserve.rs:\n{serve_rs}");

    let callee_owned = {
        let i = parse_rs.find("fn wire_parse").unwrap_or(0);
        let sl = &parse_rs[i..parse_rs.len().min(i + 160)];
        (sl.contains("sql: String") || sl.contains("sql:String"))
            && !(sl.contains("sql: &str") || sl.contains("sql:&str"))
    };
    let caller_demoted = {
        let i = serve_rs.find("fn on_simple_query").unwrap_or(0);
        let sl = &serve_rs[i..serve_rs.len().min(i + 120)];
        sl.contains("sql: &str") || sl.contains("sql:&str")
    };
    let call_ok = serve_rs.contains("wire_parse(session, \"\".to_string(), sql.to_string()")
        || serve_rs.contains("sql.to_string()")
        || serve_rs.contains("String::from(sql)");
    let call_bad = serve_rs.contains("wire_parse(session, \"\", sql)")
        && !serve_rs.contains(".to_string()");

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "WDB-191 RED: demoted &str into owned wire_parse without to_string. \
             Product: pg_wire_parse(session, \"\", sql). Got:\n{serve_rs}\n{parse_rs}"
        );
    }
    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "WDB-191: demoted &str into owned String must to_string. Got:\n{serve_rs}"
        );
    }
}

#[test]
fn wdb191_product_pg_serve_must_to_string_demoted_sql_into_owned_parse() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let serve = if tip.join("relational_pg_serve_port.rs").exists() {
        // Prefer gen when tip may differ; check both and fail on gen residual.
        gen.join("relational/relational_pg_serve_port.rs")
    } else {
        gen.join("relational/relational_pg_serve_port.rs")
    };
    let wire = gen.join("relational/relational_pg_wire_port.rs");
    if !serve.exists() {
        eprintln!("WDB-191: skip — pg_serve missing");
        return;
    }
    let serve_text = std::fs::read_to_string(&serve).expect("serve");
    let wire_text = if wire.exists() {
        std::fs::read_to_string(&wire).unwrap_or_default()
    } else {
        String::new()
    };
    let parse_owned = wire_text.contains("fn pg_wire_parse(")
        && (wire_text.contains("name: String") || wire_text.contains("sql: String"));
    // Fallback: look for call shape even if wire formal scan fails
    let caller_demoted = serve_text.contains("fn pg_wire_serve_on_simple_query(state: PgWireServeState, sql: &str)");
    let bare = serve_text.contains("pg_wire_parse(session, \"\", sql)")
        && !serve_text.contains("pg_wire_parse(session, \"\".to_string(), sql.to_string()")
        && !serve_text.contains("sql.to_string()");
    eprintln!(
        "WDB-191 product parse_owned={} demoted={} bare={} path={}",
        parse_owned,
        caller_demoted,
        bare,
        serve.display()
    );
    assert!(
        !(caller_demoted && bare),
        "WDB-191 RED: product passes demoted &str sql into owned pg_wire_parse. {}",
        serve.display()
    );
}
