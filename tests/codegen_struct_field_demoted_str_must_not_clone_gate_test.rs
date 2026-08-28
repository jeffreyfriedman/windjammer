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

//! Single-use struct field into demoted `&str` formal must borrow, not `.clone()`.
//!
//! Dogfood (`home.wj` / `tables.wj`): `document_status_counts_toward_open(inv.status)`
//! — inline trim demotes callee to `&str` but field multi-use clone heuristics
//! must not emit `status.clone()` (E0308).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn struct_field_single_use_into_demoted_str_must_borrow_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "enum_wires.wj",
        r#"
/// Inline trim demotes formal to `&str`.
pub fn document_status_counts_toward_open(status: string) -> bool {
    let s = "${status.trim().to_ascii_lowercase()}"
    s != "paid"
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
"#,
    );
    test.add_file(
        "home.wj",
        r#"
use crate::enum_wires::document_status_counts_toward_open
use crate::fields::InvoiceFields

pub fn sum_open(invoices: Vec<InvoiceFields>) -> i64 {
    let mut total: i64 = 0
    for inv in invoices {
        if document_status_counts_toward_open(inv.status) {
            total = total + inv.total_cents
        }
    }
    total
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let wires = map
        .get("enum_wires.rs")
        .expect("enum_wires.rs output");
    let home = map.get("home.rs").expect("home.rs output");

    assert!(
        wires.contains("status: &str"),
        "repro must demote read-only string formals to &str. enum_wires=\n{wires}"
    );

    assert!(
        !home.contains("inv.status.clone()"),
        "single-use struct field into demoted &str must borrow, not clone. home=\n{home}"
    );

    test.cargo_check()
        .expect("struct field demoted str call site must cargo check");
}
