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

//! demoted `&str` into owned `String` parse formal must `.to_string()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod parse
pub mod serve
"#;

const PARSE: &str = r#"
pub struct Session {
    pub n: int,
}

/// Owns name/sql strings.
pub fn wire_parse(session: Session, name: string, sql: string) -> Session {
    let _ = name.len() + sql.len()
    Session { n: session.n + 1 }
}
"#;

const SERVE: &str = r#"
use crate::parse::Session
use crate::parse::wire_parse

/// Multi-use read demotes toward `&str`.
pub fn sql_len(sql: string) -> int {
    sql.len()
}

pub fn on_simple_query(session: Session, sql: string) -> Session {
    let _ = sql_len(sql)
    wire_parse(session, "", sql)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("parse.wj", PARSE);
    test.add_file("serve.wj", SERVE);
    test
}

#[test]
fn module_file_demoted_str_into_owned_parse_must_to_string() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let parse_rs = map.get("parse.rs").expect("parse.rs");
    let serve_rs = map.get("serve.rs").expect("serve.rs");

    eprintln!("parse.rs:\n{parse_rs}\nserve.rs:\n{serve_rs}");

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
            "RED demoted &str into owned wire_parse without to_string. \
Got:\n{serve_rs}\n{parse_rs}"
        );
    }
    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "demoted &str into owned String must to_string. Got:\n{serve_rs}"
        );
    }
}
