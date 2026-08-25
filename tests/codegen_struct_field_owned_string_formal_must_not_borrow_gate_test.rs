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

//! FAILING REPRO — struct field into owned `string` formal must clone/own, not `&field`.
//!
//! Dogfood (finance-screens tip regen / `escape_html`):
//! `escape_html(link.href)` emits `&link.href` while `.replace` keeps formal `String` → E0308.
//!
//! Bool helpers that only trim/compare demote to `&str` (product P3.180) — those accept
//! `&field` and are not this gate. Owned transformers (`escape_html`, builders) still fail.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn struct_field_into_owned_string_formal_must_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "html.wj",
        r#"
pub fn escape_html(s: string) -> string {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
}
"#,
    );
    test.add_file(
        "home.wj",
        r#"
use crate::html::escape_html

pub struct Link {
    pub href: string,
}

pub fn link_html(link: Link) -> string {
    escape_html(link.href)
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map.get("home.rs").expect("home.rs output");
    let html = map.get("html.rs").expect("html.rs output");

    assert!(
        html.contains("s: String") || html.contains("s: string"),
        "repro requires owned escape_html formal (not demoted &str). Got:\n{html}"
    );
    assert!(
        !rs.contains("escape_html(&link.href)"),
        "owned string formal must not receive &link.href. Got:\n{rs}"
    );

    test.cargo_check()
        .expect("field into owned string formal must cargo check");
}
