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

//! P3.777: a `string` match scrutinee that is also passed into an `&str`
//! formal must be borrowed. Tip `wj` (2026-10-09 19:34) emits
//! `aging(&buckets, &fallback, kind)` while `kind: &str`.
//!
//! The sibling `table_html` arm in this file already borrows its strings and
//! moves the `Vec`. Finance-screens `general_ledger_table_html` is still the
//! product RED in P3.771.

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

pub fn aging(buckets: Vec<Line>, fallback: string, kind: string) -> string {
    let k = escape_html(kind)
    let f = escape_html(fallback)
    "${k}:${f}:${buckets.len()}"
}

pub struct Report {
    pub account_code: string,
    pub as_of: string,
    pub lines: Vec<Line>,
}

pub fn parse_report(json: string) -> Option<Report> {
    if json.len() == 0 {
        None
    } else {
        Some(Report {
            account_code: "1000",
            as_of: "2026",
            lines: vec![Line { n: 1 }],
        })
    }
}

pub fn parse_owned(json: string) -> Vec<Line> {
    if json.len() == 0 {
        vec![]
    } else {
        vec![Line { n: json.len() }]
    }
}

pub fn render(kind: string, json: string) -> string {
    match kind {
        "parties" => {
            let rows = parse_owned(json)
            rows.len().to_string()
        },
        "generalLedger" => {
            match parse_report(json) {
                Some(report) => {
                    let account_code = report.account_code
                    let as_of = report.as_of
                    let lines = report.lines
                    if lines.len() == 0 {
                        "empty"
                    } else {
                        table_html(account_code, as_of, lines)
                    }
                },
                None => "bad",
            }
        },
        "aging" => {
            let fallback = "${json}"
            let buckets = parse_owned("${json}")
            aging(buckets, fallback, kind)
        },
        _ => "other",
    }
}
"#;

#[test]
fn match_scrutinee_str_must_be_borrowed_at_call() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.777 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.777 lib.rs:\n{rs}");
    assert!(
        rs.contains("fn aging(") && rs.contains("kind: &str"),
        "P3.777: escape_html must demote aging's kind formal to &str:\n{rs}"
    );
    let aging = rs
        .lines()
        .find(|l| l.contains("aging(") && !l.contains("fn aging"))
        .unwrap_or("");
    assert!(
        aging.contains("&fallback") && aging.contains("&kind"),
        "P3.777: match scrutinee `kind` must be borrowed, got `{aging}`:\n{rs}"
    );
}
