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

//! P3.781: a cross-module aging call must move both `Vec`s and borrow every
//! formal that demoted to `&str`.
//!
//! Tip `wj` (2026-10-09 20:06) emits
//! `aging(buckets, &parties, fallback, title, kind.clone())` against
//! `(Vec<Bucket>, Vec<Line>, &str, &str, &str)`.
//! The sibling ledger call in the same render borrows its strings and moves
//! the `Vec`. Finance-screens `aging_report_html(&buckets, parties, fallback, title, kind)`
//! is the same class (P3.771).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const HTML: &str = r#"
pub fn escape_html(s: string) -> string {
    s.replace("&", "&amp;")
}

pub fn money_html(cents: int) -> string {
    "${cents}"
}
"#;

const TABLES: &str = r#"
use super::html::{escape_html, money_html}

pub struct Line {
    pub memo: string,
    pub amount: int,
}

pub struct Bucket {
    pub label: string,
}

pub fn ledger(account_code: string, as_of: string, ending: int, lines: Vec<Line>) -> string {
    let code = escape_html(account_code)
    let when = escape_html(as_of)
    let ending_html = money_html(ending)
    let mut n = 0
    for row in lines {
        let memo = escape_html(row.memo)
        n = n + row.amount
        let _ = memo
    }
    "${code}:${when}:${ending_html}:${n}"
}

pub fn aging(buckets: Vec<Bucket>, parties: Vec<Line>, fallback: string, title: string, kind: string) -> string {
    let head = escape_html(fallback)
    let k = escape_html(kind)
    let mut n = 0
    for row in buckets {
        let label = escape_html(row.label)
        n = n + 1
        let _ = label
    }
    for row in parties {
        n = n + row.amount
    }
    "${head}:${title}:${k}:${n}"
}
"#;

const READ: &str = r#"
use super::html::money_html
use super::tables::{Line, Bucket, ledger, aging}

pub struct Report {
    pub as_of: string,
    pub account_code: string,
    pub ending: int,
    pub lines: Vec<Line>,
}

pub fn parse(json: string) -> Option<Report> {
    if json.len() == 0 {
        None
    } else {
        Some(Report {
            as_of: "d",
            account_code: "a",
            ending: 1,
            lines: vec![Line { memo: "m", amount: 1 }],
        })
    }
}

pub fn parse_buckets(json: string) -> Vec<Bucket> {
    if json.len() == 0 {
        vec![]
    } else {
        vec![Bucket { label: "cur" }]
    }
}

pub fn parse_parties(json: string) -> Vec<Line> {
    if json.len() == 0 {
        vec![]
    } else {
        vec![Line { memo: "p", amount: 2 }]
    }
}

pub fn title_from(_json: string) -> string {
    "t"
}

pub fn kind_from(_json: string) -> string {
    "ar"
}

pub fn render(kind: string, json: string) -> string {
    match kind {
        "generalLedger" => {
            match parse(json) {
                Some(report) => {
                    let account_code = report.account_code
                    let as_of = report.as_of
                    let ending_balance_cents = report.ending
                    let lines = report.lines
                    if lines.len() == 0 {
                        let ending = money_html(ending_balance_cents)
                        "empty ${ending}"
                    } else {
                        ledger(account_code, as_of, ending_balance_cents, lines)
                    }
                },
                None => "bad",
            }
        },
        "aging" => {
            let fallback = "${json}"
            let title = title_from("${json}")
            let wire = kind_from("${json}")
            let buckets = parse_buckets("${json}")
            let parties = parse_parties(json)
            aging(buckets, parties, fallback, title, wire)
        },
        _ => "other",
    }
}
"#;

#[test]
fn cross_module_aging_must_move_vecs_and_borrow_demoted_strs() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "mod html;\nmod tables;\nmod read;\n");
    test.add_file("html.wj", HTML);
    test.add_file("tables.wj", TABLES);
    test.add_file("read.wj", READ);
    let map = test.compile().expect("P3.781 compile");
    let tables = map.get("tables.rs").expect("tables.rs");
    let read = map.get("read.rs").expect("read.rs");
    eprintln!("P3.781 tables.rs:\n{tables}\nP3.781 read.rs:\n{read}");
    assert!(
        tables.contains("parties: Vec<") && tables.contains("fallback: &str") && tables.contains("kind: &str"),
        "P3.781: aging must keep both Vecs owned and demote fallback/kind:\n{tables}"
    );
    let call = read
        .lines()
        .find(|l| l.contains("aging(") && !l.contains("fn aging"))
        .unwrap_or("");
    assert!(
        !call.contains("&parties")
            && !call.contains("&buckets")
            && call.contains("&fallback")
            && call.contains("&title")
            && call.contains("&wire")
            && !call.contains(".clone()"),
        "P3.781 RED: cross-module aging borrowed the wrong args, got `{call}`:\n{read}"
    );
}
