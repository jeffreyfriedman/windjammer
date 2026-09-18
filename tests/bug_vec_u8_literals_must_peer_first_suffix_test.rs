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

//! P3.368: `vec![1_u8, 2, 3]` must not suffix untyped peers as `_i32` in i32-coord fns.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod buf
"#;

const BUF: &str = r#"
pub fn sample() -> Vec<u8> {
    vec![1_u8, 2, 3, 4]
}
"#;

#[test]
fn vec_u8_literals_must_peer_first_suffix() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("buf.wj", BUF);
    let map = test.compile().expect("P3.368 vec compile");
    let rs = map.get("buf.rs").expect("buf.rs");
    assert!(
        !rs.contains("2_i32") && !rs.contains("3_i32"),
        "P3.368: vec![u8, …] must not emit _i32 peers:\n{rs}"
    );
    assert!(rs.contains("2_u8") || rs.contains("vec![1_u8, 2, 3"), "expected u8 peers:\n{rs}");
    test.cargo_check().expect("P3.368 vec cargo-check");
}
