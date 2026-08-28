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

//! FAILING REPRO — cross-module struct field into demoted `&str` formal must borrow,
//! not emit `field + ""` temp (`format!("{}{}", field, "")`).
//!
//! Dogfood (`home.wj`): `document_status_counts_toward_open(inv.status)` borrows
//! (`&inv.status`) but `bank_line_is_unmatched(line.status)` in same function still
//! emits owned re-own temp when callee lives in `tables` not `enum_wires`.
//!
//! Sibling: `codegen_struct_field_demoted_str_must_not_clone_gate_test` (same-crate GREEN).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn cross_module_struct_field_demoted_str_must_borrow_not_plus_empty() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "enum_wires.wj",
        r#"
pub fn document_status_counts_toward_open(status: string) -> bool {
    let s = "${status.trim().to_ascii_lowercase()}"
    s != "paid"
}
"#,
    );
    test.add_file(
        "tables.wj",
        r#"
pub fn bank_line_is_unmatched(status: string) -> bool {
    let s = "${status.trim().to_ascii_lowercase()}"
    s != "matched"
}
"#,
    );
    test.add_file(
        "fields.wj",
        r#"
pub struct InvoiceFields {
    pub status: string,
    pub total_cents: i64,
}

pub struct BankLineFields {
    pub status: string,
}
"#,
    );
    test.add_file(
        "home.wj",
        r#"
use crate::enum_wires::document_status_counts_toward_open
use crate::tables::bank_line_is_unmatched
use crate::fields::{InvoiceFields, BankLineFields}

pub fn sum_open_and_unmatched(invoices: Vec<InvoiceFields>, bank: Vec<BankLineFields>) -> i64 {
    let mut total: i64 = 0
    for inv in invoices {
        if document_status_counts_toward_open(inv.status) {
            total = total + inv.total_cents
        }
    }
    for line in bank {
        if bank_line_is_unmatched(line.status) {
            total = total + 1
        }
    }
    total
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let home = map.get("home.rs").expect("home.rs output");

    assert!(
        home.contains("document_status_counts_toward_open(&inv.status)"),
        "same-crate demoted str should borrow. home=\n{home}"
    );

    assert!(
        home.contains("bank_line_is_unmatched(&line.status)")
            && !home.contains("format!(\"{}{}\", line.status, \"\")"),
        "cross-module demoted str must borrow, not + \"\" temp. home=\n{home}"
    );

    test.cargo_check()
        .expect("cross-module demoted str call site must cargo check");
}
