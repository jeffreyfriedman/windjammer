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

//! FAILING REPRO — struct field moved into owned `string` formal must auto-clone
//! when the same field is reused (format / second call).
//!
//! Dogfood (`panels.wj`): `escape_html(row.account_code)` then
//! `"${row.account_code} · ${row.account_name}"` → tip emits move then borrow (E0382).
//! Product workaround: `row.account_code + ""` before escape.
//!
//! Sibling of `codegen_multi_use_owned_param_must_auto_clone` (params GREEN);
//! this gate covers **field** multi-use.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn multi_use_struct_field_must_clone_before_owned_formal() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "html.wj",
        r#"
pub fn escape_html(s: string) -> string {
    s.replace("&", "&amp;")
}
"#,
    );
    test.add_file(
        "row.wj",
        r#"
pub struct Row {
    pub account_code: string,
    pub account_name: string,
}
"#,
    );
    test.add_file(
        "panels.wj",
        r#"
use crate::html::escape_html
use crate::row::Row

pub fn label_for(row: Row) -> string {
    let code = escape_html(row.account_code)
    let label_raw = "${row.account_code} · ${row.account_name}"
    escape_html(label_raw) + code
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map.get("panels.rs").expect("panels.rs output");

    let has_clone = rs.contains("account_code.clone()") || rs.contains(".clone()");
    assert!(
        has_clone,
        "struct field multi-use must auto-clone before owned formal. Got:\n{rs}"
    );

    test.cargo_check()
        .expect("field multi-use must cargo check");
}
