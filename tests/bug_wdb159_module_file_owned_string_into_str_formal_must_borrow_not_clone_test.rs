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

//! WDB-159: owned `string` into demoted `&str` formal must borrow, not `.clone()`.
//!
//! Product dispatch: `relational_sql_parse_*_front(sql.clone())` where formal is `&str`
//! → E0308 expected `&str`, found `String` (~158 diagnostics).
//!
//! Use `string` in `.wj` (W0010). Tip may demote read-only formals to `&str`; call sites
//! that reuse the local must emit `&sql`, never `sql.clone()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod parse
pub mod dispatch
"#;

const PARSE: &str = r#"
pub fn parse_front(sql: string) -> bool {
    sql.len() > 0
}
"#;

const DISPATCH: &str = r#"
use crate::parse::parse_front

pub fn dispatch_sql(sql: string) -> bool {
    let first = parse_front(sql)
    let second = parse_front(sql)
    first || second
}
"#;

fn wdb159_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("parse.wj", PARSE);
    test.add_file("dispatch.wj", DISPATCH);
    test
}

#[test]
fn wdb159_module_file_owned_string_into_str_formal_must_borrow_not_clone() {
    let test = wdb159_fixture();
    let map = test
        .compile()
        .expect("WDB-159 multipass compile should succeed (codegen may still be wrong)");
    let parse_rs = map.get("parse.rs").expect("parse.rs");
    let dispatch_rs = map.get("dispatch.rs").expect("dispatch.rs");

    let demoted = parse_rs.contains("sql: &str") || parse_rs.contains("sql:&str");
    let cloned_into = dispatch_rs.contains("parse_front(sql.clone())");

    eprintln!("WDB-159 parse.rs:\n{parse_rs}\ndispatch.rs:\n{dispatch_rs}");

    if demoted && cloned_into {
        panic!(
            "WDB-159 RED: demoted &str formal received sql.clone() (String). \
             Product: relational_sql_parse_*_front(sql.clone()) → E0308 expected &str, found String."
        );
    }

    // When tip demotes to &str, reuse must borrow (`&sql`), not clone-to-String.
    if demoted {
        let borrowed = dispatch_rs.contains("parse_front(&sql)");
        assert!(
            borrowed,
            "WDB-159 RED: demoted &str formal needs &sql on reused local. Product dispatch front parsers."
        );
    }
}
