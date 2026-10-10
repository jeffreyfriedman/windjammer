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

//! Regression: module file strings contains literal must match demoted str formal.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod check
"#;

const CHECK: &str = r#"
use windjammer_runtime::strings

pub fn classify_label(label: string) -> string {
    let mut service = "unknown"
    if strings::contains(label, "service=checkout") {
        service = "checkout"
    }
    if strings::contains(label, "attr=42") {
        service = "with-attr"
    }
    service
}

pub fn cap() -> string {
    classify_label("service=checkout attr=42")
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("check.wj", CHECK);
    test
}

#[test]
fn module_file_strings_contains_literal_must_match_demoted_str_formal() {
    let mut test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let rs = map.get("check.rs").expect("check.rs must be generated");

    let bad_owned_lit = rs.contains("String::from(\"service=checkout\")")
        || rs.contains("String::from(\"attr=42\")")
        || rs.contains("\"service=checkout\".to_string()")
        || rs.contains("\"attr=42\".to_string()");
    if bad_owned_lit {
        eprintln!("RED emit check.rs:\n{rs}");
    }

    test.cargo_check().expect(
        "RED: reused-label strings::contains literals must stay &str when formal is &str.",
    );
}
