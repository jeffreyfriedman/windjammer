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

//!
//! body does `sql.clone().into()` → E0599 (clone not on impl Into<String>).
//! Windjammer source uses `sql: string` — the compiler should emit `String` (or `&str` + to_string),
//! never bare `impl Into<String>` with clone reuse.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod session
pub mod parse
"#;

const SESSION: &str = r#"
pub struct WireSession {
    pub last_sql: string
    pub last_name: string
}
"#;

const PARSE: &str = r#"
use crate::session::WireSession

pub fn wire_parse(session: WireSession, name: string, sql: string) -> WireSession {
    WireSession {
        last_sql: sql,
        last_name: name,
    }
}

pub fn wire_parse_reuse(session: WireSession, name: string, sql: string) -> WireSession {
    let s1 = wire_parse(session, name, sql)
    // reuse name+sql after move would need clone in WJ — keep single use in body via fields
    s1
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("session.wj", SESSION);
    test.add_file("parse.wj", PARSE);
    test
}

#[test]
fn module_file_string_formal_must_not_emit_impl_into_string_with_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let parse_rs = map.get("parse.rs").expect("parse.rs");

    let into_formal = parse_rs.contains("impl Into<String>") || parse_rs.contains("impl Into<string>");
    let clone_into = parse_rs.contains("sql.clone()") && parse_rs.contains("Into<String>");

    if into_formal || clone_into {
        eprintln!("RED parse.rs:\n{parse_rs}");
    }

    assert!(
        !into_formal,
        "RED: string formal must not emit `impl Into<String>`. "
    );
}
