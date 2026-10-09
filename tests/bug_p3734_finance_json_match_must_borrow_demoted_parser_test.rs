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

//! P3.734: finance-screens `json.wj` parsers that demote to `&str` must be
//! borrowed from a match that also passes the same `json` to owned parsers.
//! The checked-in `build/read_models.rs` still has `json.clone()` / bare
//! `String` at those calls. This gate compiles the WJ sources with the
//! in-process compiler.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

fn finance_json_wj() -> PathBuf {
    let mut walked = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let candidate = walked.join(
            "financial-management/financial-management-platform/packages/finance-screens/src/json.wj",
        );
        if candidate.is_file() {
            return candidate;
        }
        let candidate = walked.join(
            "financial-management-platform/packages/finance-screens/src/json.wj",
        );
        if candidate.is_file() {
            return candidate;
        }
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    panic!("P3.734: finance-screens src/json.wj not found");
}

const CALLER: &str = r#"
use crate::json::{parse_balance_sheet_fields, parse_party_fields}

pub fn render(kind: string, json: string) -> int {
    match kind {
        "parties" => parse_party_fields(json).len(),
        "sheet" => match parse_balance_sheet_fields(json) {
            Some(sheet) => sheet.assets_cents,
            None => 0,
        },
        _ => 0,
    }
}
"#;

#[test]
fn finance_json_match_must_borrow_demoted_balance_sheet_parser() {
    let json_src = std::fs::read_to_string(finance_json_wj()).expect("json.wj");
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "pub mod json\npub mod caller\n");
    test.add_file("json.wj", &json_src);
    test.add_file("caller.wj", CALLER);
    let map = test
        .compile()
        .expect("P3.734 finance json.wj must transpile");
    let caller = map
        .get("caller.rs")
        .unwrap_or_else(|| panic!("caller.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    let json_rs = map.get("json.rs").expect("json.rs");
    eprintln!("P3.734 caller.rs:\n{caller}");
    let demoted = json_rs.contains("fn parse_balance_sheet_fields(json: &str)");
    assert!(
        demoted,
        "P3.734: expected parse_balance_sheet_fields to demote to &str; got signature area missing"
    );
    let bad = caller.contains("parse_balance_sheet_fields(json.clone())")
        || (caller.contains("parse_balance_sheet_fields(json)")
            && !caller.contains("parse_balance_sheet_fields(&json)"));
    assert!(
        !bad,
        "P3.734 RED: demoted balance-sheet parser fed owned String:\n{caller}"
    );
}
