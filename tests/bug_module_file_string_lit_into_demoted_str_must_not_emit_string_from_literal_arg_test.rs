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

//! DB-168: string literal into demoted `&str` formal must stay `&str`, not `String::from`.
//!
//! Product (`wave1_opt_hardware_port.wj`):
//!   session = pg_wire_parse(session, "s1", wave1_opt_point_select_sql())
//! Full multipass emits:
//!   pg_wire_parse(session, String::from("s1"), …) while formal is `name: &str`
//! → E0308 expected `&str`, found `String` (large share of ~162 &str←String residuals).
//!
//! Expected: emit `"s1"` (or `&…`) into demoted `&str`. Keep owned `String` formal is also GREEN.
//! Distinct from DB-152 (lit → owned `string` needs `.to_string()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("opt.wj", OPT);
    test
}

#[test]
fn module_file_string_lit_into_demoted_str_must_not_emit_string_from() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let opt_rs = map.get("opt.rs").expect("opt.rs");

    eprintln!("wire.rs:\n{wire_rs}\nopt.rs:\n{opt_rs}");

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
            "demoted &str name formal received String::from(\"s1\"). \
             Product: wave1_opt pg_wire_parse(session, String::from(\"s1\"), …). Got:\n{opt_rs}\n{wire_rs}"
        );
    }

    if demoted_name {
        assert!(
            opt_rs.contains("pg_wire_parse(session, \"s1\"")
                || opt_rs.contains("pg_wire_parse(session,\"s1\"")
                || opt_rs.contains("&String::from(\"s1\")")
                || opt_rs.contains("pg_wire_parse(session, &"),
            "demoted &str must keep lit as &str. Got:\n{opt_rs}"
        );
    }
}
