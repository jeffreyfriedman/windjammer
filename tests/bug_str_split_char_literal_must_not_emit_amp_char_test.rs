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

//! Regression: str split char literal must not emit amp char.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn file_ext(path: string) -> string {
    if let Some(ext) = path.split('.').last() {
        return ext
    }
    ""
}
"#;

#[test]
fn str_split_char_literal_must_not_emit_amp_char() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.402 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.402 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("split(&'.')") && !rs.contains("split(&\".\")"),
        "P3.402 RED: split char literal must not emit &char / &str-ref:\n{rs}"
    );
    assert!(
        rs.contains("split('.')") || rs.contains("split(\".\")"),
        "P3.402: expected bare char/str split delimiter:\n{rs}"
    );
    test.cargo_check().expect("P3.402 cargo-check");
}
