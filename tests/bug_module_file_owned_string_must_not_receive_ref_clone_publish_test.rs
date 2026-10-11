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

//! owned `String` formals must not receive `&dated_label.clone()` (wave1 publish).
//!
//! Product tip-out/gen:
//!   `wave1_publish_append_gate_rows(&out, …, &dated_label.clone(), …)`
//!   with `dated_label: String` → E0308.
//! Twin of ; prefer `dated_label.clone()` without `&` (and owned `out` move/clone).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn append_gate_rows(out: string, dated_label: string) -> string {
    out + "|" + dated_label
}

pub fn format_check(dated_label: string) -> string {
    let mut out = "hdr"
    out = append_gate_rows(out, dated_label)
    out
}
"#;

#[test]
fn module_file_owned_string_must_not_receive_ref_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn append_gate_rows(out: String")
        || rs.contains("fn append_gate_rows(mut out: String");
    assert!(
        owned,
        "expected owned String formals on append_gate_rows:\n{rs}"
    );
    let bad = rs.contains("&dated_label.clone()")
        || rs.contains("&out.clone()")
        || rs.contains("append_gate_rows(&");
    assert!(
        !bad,
        "owned append_gate_rows received &owned.clone() / &args:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
