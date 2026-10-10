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

//! i32 coord `cy + N` must not emit `N_i64 as i32`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn offset_y(cy: i32, delta: i32) -> i32 {
    cy + 3
}

pub fn place(cy: i32) -> i32 {
    offset_y(cy, 0) + 2
}
"#;

#[test]
fn module_file_i32_coord_add_must_not_emit_i64_as_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("_i64 as i32")
        || rs.contains("3_i64")
        || rs.contains("2_i64");
    assert!(
        !bad,
        "RED i32 coord add emitted i64 lit / as i32:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
