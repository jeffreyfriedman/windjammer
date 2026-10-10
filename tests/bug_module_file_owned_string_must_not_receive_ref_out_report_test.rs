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

//! Regression: module file owned string must not receive ref out report.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn append_bool(out: string, value: bool) -> string {
    if value {
        return out + "|true"
    }
    out + "|false"
}

pub fn format_flags(live: bool, competitive: bool) -> string {
    let mut out = "hdr"
    out = append_bool(out, live)
    out = append_bool(out, competitive)
    out
}
"#;

#[test]
fn module_file_owned_string_must_not_receive_ref_out() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn append_bool(out: String")
        || rs.contains("fn append_bool(mut out: String");
    assert!(
        owned,
        "expected owned String formal on append_bool:\n{rs}"
    );
    let bad = rs.contains("append_bool(&out")
        || rs.contains("append_bool(&mut out");
    assert!(
        !bad,
        "RED: owned append_bool received &out:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
