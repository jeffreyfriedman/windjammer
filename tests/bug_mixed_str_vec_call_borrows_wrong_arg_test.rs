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

//! a call that mixes demoted `&str` formals with an owned `Vec` formal
//! must borrow only the strings.
//!
//! The compiler still emitted
//! `general_ledger_table_html(account_code, as_of.clone(), ending, &lines)`
//! against `(&str, &str, i64, Vec<...>)`, and
//! `aging_report_html(&buckets, parties, fallback, title, kind)` against
//! `(Vec<...>, Vec<...>, &str, String, &str)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const TABLES: &str = r#"
pub fn escape_html(s: string) -> string {
    s
}

pub struct Line {
    pub n: int,
}

pub fn table_html(account_code: string, as_of: string, lines: Vec<Line>) -> int {
    let code = escape_html(account_code)
    let when = escape_html(as_of)
    let mut n = 0
    for row in lines {
        n = n + row.n
    }
    code.len() + when.len() + n
}
"#;

const READ: &str = r#"
use super::tables::table_html

pub struct Report {
    pub account_code: string,
    pub as_of: string,
    pub lines: Vec<super::tables::Line>,
}

pub fn render(report: Report) -> int {
    let account_code = report.account_code
    let as_of = report.as_of
    let lines = report.lines
    table_html(account_code, as_of, lines)
}
"#;

#[test]
fn mixed_str_and_owned_vec_call_must_borrow_strings_only() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "mod tables;\nmod read;\n");
    test.add_file("tables.wj", TABLES);
    test.add_file("read.wj", READ);
    let map = test.compile().expect("compile");
    let tables = map.get("tables.rs").expect("tables.rs");
    let read = map.get("read.rs").expect("read.rs");
    eprintln!("tables.rs:\n{tables}\nP3.771 read.rs:\n{read}");
    let demoted = tables.contains("account_code: &str") && tables.contains("lines: Vec<");
    if !demoted {
        eprintln!("isolate: strings were not demoted beside an owned Vec");
        return;
    }
    let call = read
        .lines()
        .find(|l| l.contains("table_html("))
        .unwrap_or(read);
    assert!(
        call.contains("&account_code") && call.contains("&as_of") && !call.contains("&lines"),
        "RED: mixed &str/Vec call borrowed the wrong args:\n{read}"
    );
}

