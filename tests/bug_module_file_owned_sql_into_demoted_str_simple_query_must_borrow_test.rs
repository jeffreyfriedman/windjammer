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

//! owned `String` sql into demoted `&str` `on_simple_query` must borrow.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod serve
pub mod unified
"#;

const SERVE: &str = r#"
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
    on_simple_query(state, sql)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("serve.wj", SERVE);
    test.add_file("unified.wj", UNIFIED);
    test
}

#[test]
fn module_file_owned_sql_into_demoted_str_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let serve_rs = map.get("serve.rs").expect("serve.rs");
    let unified_rs = map.get("unified.rs").expect("unified.rs");
    eprintln!("isolate serve.rs:\n{serve_rs}\nunified.rs:\n{unified_rs}");
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
            "RED demoted &str received owned sql. Got:\n{unified_rs}\n{serve_rs}"
        );
    }
    test.cargo_check()
        .expect("owned sql into demoted &str must cargo-check");
}
