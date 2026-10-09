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

//! P3.773: `replace` demotes a `string` formal to `&str`. A later call that
//! also moves a `Vec` must borrow only those strings.
//!
//! Finance-screens `escape_html` is `&str` via `replace`, and
//! `general_ledger_table_html` is `(&str, &str, i64, Vec<...>)`, but the call
//! still passes owned strings and `&lines`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn escape_html(s: string) -> string {
    s.replace("&", "&amp;").replace("<", "&lt;")
}

pub struct Line {
    pub n: int,
}

pub fn table_html(account_code: string, as_of: string, lines: Vec<Line>) -> string {
    let code = escape_html(account_code)
    let when = escape_html(as_of)
    let mut n = 0
    for row in lines {
        n = n + row.n
    }
    "${code}:${when}:${n}"
}

pub struct Report {
    pub account_code: string,
    pub as_of: string,
    pub lines: Vec<Line>,
}

pub fn render(report: Report) -> string {
    let account_code = report.account_code
    let as_of = report.as_of
    let lines = report.lines
    if lines.len() == 0 {
        "empty"
    } else {
        table_html(account_code, as_of, lines)
    }
}
"#;

#[test]
fn replace_demoted_str_beside_owned_vec_must_borrow_strings_only() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.773 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.773 lib.rs:\n{rs}");
    assert!(
        rs.contains("fn escape_html(s: &str)"),
        "P3.773: replace chain must demote escape_html to &str:\n{rs}"
    );
    assert!(
        rs.contains("account_code: &str") && rs.contains("lines: Vec<"),
        "P3.773: table_html must demote strings and keep the Vec owned:\n{rs}"
    );
    let call = rs
        .lines()
        .find(|l| l.contains("table_html(") && !l.contains("fn table_html"))
        .unwrap_or(rs);
    assert!(
        call.contains("&account_code") && call.contains("&as_of") && !call.contains("&lines"),
        "P3.773 RED: mixed &str/Vec call borrowed the wrong args:\n{call}\n{rs}"
    );
}
