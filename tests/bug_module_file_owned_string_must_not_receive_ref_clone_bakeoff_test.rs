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

//! Regression: module file owned string must not receive ref clone bakeoff.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Fill {
    pub hardware: string,
    pub scale: string,
    pub label: string,
}

pub fn bakeoff_run_to_fill(hardware: string, scale: string, dated_label: string) -> Fill {
    Fill {
        hardware: hardware,
        scale: scale,
        label: dated_label,
    }
}

pub fn fill_bundle(hw: string, tpch_scale: string, label: string) -> Fill {
    let f1 = bakeoff_run_to_fill(hw, tpch_scale, label)
    let _f2 = bakeoff_run_to_fill(hw, tpch_scale, label)
    f1
}
"#;

#[test]
fn module_file_owned_string_must_not_receive_ref_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn bakeoff_run_to_fill(hardware: String")
        || rs.contains("fn bakeoff_run_to_fill(mut hardware: String");
    assert!(
        owned,
        "expected owned String formals on bakeoff_run_to_fill:\n{rs}"
    );
    let bad = rs.contains("&hw.clone()")
        || rs.contains("&tpch_scale.clone()")
        || rs.contains("&label.clone()")
        || rs.contains("bakeoff_run_to_fill(&");
    assert!(
        !bad,
        "RED: owned bakeoff_run_to_fill received &owned.clone() / &args:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
