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

//! P3.771: a call that mixes demoted `&str` formals with an owned `Vec` formal
//! must borrow only the strings.
//!
//! Oct 9 tip `wj` regen of finance-screens still emits
//! `general_ledger_table_html(account_code, as_of.clone(), ending, &lines)`
//! against `(&str, &str, i64, Vec<...>)`, and
//! `aging_report_html(&buckets, parties, fallback, title, kind)` against
//! `(Vec<...>, Vec<...>, &str, String, &str)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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

fn finance_build(name: &str) -> Option<PathBuf> {
    let mut walked = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        for rel in [
            "financial-management/financial-management-platform/packages/finance-screens/build",
            "financial-management-platform/packages/finance-screens/build",
        ] {
            let path = walked.join(rel).join(name);
            if path.is_file() {
                return Some(path);
            }
        }
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

#[test]
fn mixed_str_and_owned_vec_call_must_borrow_strings_only() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "mod tables;\nmod read;\n");
    test.add_file("tables.wj", TABLES);
    test.add_file("read.wj", READ);
    let map = test.compile().expect("P3.771 compile");
    let tables = map.get("tables.rs").expect("tables.rs");
    let read = map.get("read.rs").expect("read.rs");
    eprintln!("P3.771 tables.rs:\n{tables}\nP3.771 read.rs:\n{read}");
    let demoted = tables.contains("account_code: &str") && tables.contains("lines: Vec<");
    if !demoted {
        eprintln!("P3.771 isolate: strings were not demoted beside an owned Vec");
        return;
    }
    let call = read
        .lines()
        .find(|l| l.contains("table_html("))
        .unwrap_or(read);
    assert!(
        call.contains("&account_code") && call.contains("&as_of") && !call.contains("&lines"),
        "P3.771 RED: mixed &str/Vec call borrowed the wrong args:\n{read}"
    );
}

#[test]
fn finance_screens_mixed_str_vec_calls_must_match_formals() {
    let read_path = finance_build("read_models.rs").expect("finance-screens build/read_models.rs");
    let tables_path = finance_build("tables.rs").expect("finance-screens build/tables.rs");
    let read = std::fs::read_to_string(&read_path).expect("read_models");
    let tables = std::fs::read_to_string(&tables_path).expect("tables");
    let gl_sig_demoted = tables.contains("fn general_ledger_table_html(account_code: &str")
        && tables.contains("lines: Vec<GeneralLedgerLineFields>");
    let gl_call_bad = read.contains("general_ledger_table_html(account_code, as_of.clone()")
        && read.contains("&lines");
    let aging_sig = tables.contains("fn aging_report_html(buckets: Vec<")
        && tables.contains("fallback_json: &str");
    let aging_call_bad = read.contains("aging_report_html(&buckets,");
    assert!(
        gl_sig_demoted && aging_sig,
        "P3.771: expected demoted finance-screens signatures in {}",
        tables_path.display()
    );
    assert!(
        !gl_call_bad && !aging_call_bad,
        "P3.771 RED: finance-screens borrows the Vec and leaves String args owned in {}",
        read_path.display()
    );
}
