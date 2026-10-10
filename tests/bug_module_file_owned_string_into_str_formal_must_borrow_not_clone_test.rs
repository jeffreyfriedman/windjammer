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

//! owned `string` into demoted `&str` formal must borrow (`&sql`), not `.clone()`.
//!
//! Multipass: several read-only fronts demote to `&str`; dispatch keeps owned `String` and
//! probes sequentially — each call must be `parse_*(&sql)`, never `sql.clone()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod parse
pub mod dispatch
"#;

const PARSE: &str = r#"
pub fn parse_a_front(sql: string) -> bool {
    sql.len() > 0
}

pub fn parse_b_front(sql: string) -> bool {
    sql.len() > 1
}

pub fn parse_c_front(sql: string) -> bool {
    sql.len() > 2
}
"#;

const DISPATCH: &str = r#"
use crate::parse::parse_a_front
use crate::parse::parse_b_front
use crate::parse::parse_c_front

pub fn dispatch_kind(sql: string) -> string {
    let mut tag = "unknown"
    if parse_a_front(sql) {
        tag = "a"
    } else if parse_b_front(sql) {
        tag = "b"
    } else if parse_c_front(sql) {
        tag = "c"
    }
    let mut out = sql
    out = out + tag
    out
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("parse.wj", PARSE);
    test.add_file("dispatch.wj", DISPATCH);
    test
}

#[test]
fn module_file_owned_string_into_str_formal_must_borrow_not_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let parse_rs = map.get("parse.rs").expect("parse.rs");
    let dispatch_rs = map.get("dispatch.rs").expect("dispatch.rs");

    let fronts_demoted = parse_rs.matches("sql: &str").count() >= 2
        || parse_rs.matches("sql:&str").count() >= 2;
    let dispatch_owned = dispatch_rs.contains("sql: String") || dispatch_rs.contains("sql:String");
    let cloned_into = dispatch_rs.contains("sql.clone()");
    let borrow_a = dispatch_rs.contains("parse_a_front(&sql)");
    let borrow_b = dispatch_rs.contains("parse_b_front(&sql)");
    let borrow_c = dispatch_rs.contains("parse_c_front(&sql)");

    eprintln!("parse.rs:\n{parse_rs}\ndispatch.rs:\n{dispatch_rs}");

    assert!(
        fronts_demoted,
        "expected compiler to demote read-only parse_*_front formals to &str."
    );
    assert!(
        dispatch_owned,
        "dispatch_kind must keep owned String."
    );

    if cloned_into {
        panic!(
            "RED owned String probed into demoted &str fronts via sql.clone(). \
"
        );
    }

    assert!(
        borrow_a && borrow_b && borrow_c,
        "RED sequential demoted &str fronts need &sql on each probe."
    );
}
