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

//! Regression: module file owned string first formal must not receive ref out publish.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn append_gate_rows(out: string, label: string) -> string {
    out + "|" + label
}

pub fn format_check(label: string) -> string {
    let mut out = "hdr"
    out = append_gate_rows(out, label)
    out
}
"#;

#[test]
fn module_file_owned_string_first_formal_must_not_receive_ref_out() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn append_gate_rows(out: String")
        || rs.contains("fn append_gate_rows(mut out: String");
    assert!(
        owned,
        "expected owned String first formal on append_gate_rows:\n{rs}"
    );
    let bad = rs.contains("append_gate_rows(&out") || rs.contains("append_gate_rows(&mut out");
    assert!(
        !bad,
        "RED: owned append_gate_rows received &out as first arg:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
