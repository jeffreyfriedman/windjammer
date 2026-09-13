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

//! WDB-168: string literal into demoted `&str` formal must stay `&str`, not `String::from`.
//!
//! Product (`wave1_opt_hardware_port.wj`):
//!   session = pg_wire_parse(session, "s1", wave1_opt_point_select_sql())
//! Full multipass emits:
//!   pg_wire_parse(session, String::from("s1"), …) while formal is `name: &str`
//! → E0308 expected `&str`, found `String` (large share of ~162 &str←String residuals).
//!
//! Expected: emit `"s1"` (or `&…`) into demoted `&str`. Keep owned `String` formal is also GREEN.
//! Distinct from WDB-152 (lit → owned `string` needs `.to_string()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod wire
pub mod opt
"#;

const WIRE: &str = r#"
pub struct Session {
    pub ready: bool,
}

/// Read-only name probe demotes to `&str` (product pg_wire_parse name formal).
pub fn pg_wire_parse(session: Session, name: string, sql: string) -> Session {
    let mut out = session
    out.ready = name.len() > 0 && sql.len() > 0
    out
}
"#;

const OPT: &str = r#"
use crate::wire::Session
use crate::wire::pg_wire_parse

pub fn point_select_sql() -> string {
    "SELECT 1"
}

pub fn run_parse(session: Session) -> Session {
    // Product wave1_opt_hardware_port: pg_wire_parse(session, "s1", …)
    pg_wire_parse(session, "s1", point_select_sql())
}
"#;

fn wdb168_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("opt.wj", OPT);
    test
}

#[test]
fn wdb168_module_file_string_lit_into_demoted_str_must_not_emit_string_from() {
    let test = wdb168_fixture();
    let map = test
        .compile()
        .expect("WDB-168 multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let opt_rs = map.get("opt.rs").expect("opt.rs");

    eprintln!("WDB-168 wire.rs:\n{wire_rs}\nopt.rs:\n{opt_rs}");

    let demoted_name = {
        let i = wire_rs.find("fn pg_wire_parse").unwrap_or(0);
        let sl = &wire_rs[i..wire_rs.len().min(i + 160)];
        sl.contains("name: &str") || sl.contains("name:&str")
    };
    let string_from_into_str = demoted_name
        && (opt_rs.contains("pg_wire_parse(session, String::from(\"s1\")")
            || opt_rs.contains("pg_wire_parse(session,String::from(\"s1\")")
            || (opt_rs.contains("String::from(\"s1\")")
                && opt_rs.contains("pg_wire_parse")
                && !opt_rs.contains("pg_wire_parse(session, \"s1\"")));

    if string_from_into_str {
        panic!(
            "WDB-168 RED: demoted &str name formal received String::from(\"s1\"). \
             Product: wave1_opt pg_wire_parse(session, String::from(\"s1\"), …). Got:\n{opt_rs}\n{wire_rs}"
        );
    }

    if demoted_name {
        assert!(
            opt_rs.contains("pg_wire_parse(session, \"s1\"")
                || opt_rs.contains("pg_wire_parse(session,\"s1\"")
                || opt_rs.contains("&String::from(\"s1\")")
                || opt_rs.contains("pg_wire_parse(session, &"),
            "WDB-168: demoted &str must keep lit as &str. Got:\n{opt_rs}"
        );
    }
}

#[test]
fn wdb168_product_wave1_opt_must_not_string_from_into_demoted_parse_name() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let wire = root.join("relational/relational_pg_wire_port.rs");
    let opt = root.join("relational/wave1_opt_hardware_port.rs");
    if !wire.exists() || !opt.exists() {
        eprintln!("WDB-168: skip product gate — wire/opt missing");
        return;
    }
    let wire_text = std::fs::read_to_string(&wire).expect("wire");
    let opt_text = std::fs::read_to_string(&opt).expect("opt");
    let demoted = wire_text.contains("pub fn pg_wire_parse(session: PgWireSession, name: &str");
    let owned_ctor_into_demoted = demoted
        && (opt_text.contains("pg_wire_parse(session, String::from(\"s1\")")
            || opt_text.contains("pg_wire_parse(session, \"s1\".to_string()"));
    eprintln!(
        "WDB-168 product demoted_name={} owned_ctor_into_demoted={}",
        demoted, owned_ctor_into_demoted
    );
    assert!(
        !owned_ctor_into_demoted,
        "WDB-168 RED: product wave1_opt still emits String::from/\"s1\".to_string() into demoted &str parse name."
    );
}
