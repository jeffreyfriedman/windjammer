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

//! Regression: module file u32 loop init must not emit 0 usize.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn walk_degree(degree: u32) -> u32 {
    let mut i = 0
    let mut acc: u32 = 0
    while i < degree {
        acc = acc + i
        i = i + 1
    }
    acc
}
"#;

#[test]
fn module_file_u32_loop_init_must_not_emit_0_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("u32 = 0_usize") || rs.contains(": u32 = 0_usize");
    assert!(
        !bad,
        "RED: MultiFile emitted u32 = 0_usize (must be 0_u32):\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
