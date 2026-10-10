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

//! `usize` accumulator must not add bare `_i32` literals (pg_wire field walk).
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn skip_fields(frame_offset: usize, field_index: usize, count: usize) -> usize {
    let mut pos = frame_offset + 7
    let mut i = 0
    while i < count {
        if i == field_index {
            return pos
        }
        pos = pos + 4 + 2 + 4 + 2 + 4 + 2
        i = i + 1
    }
    pos
}
"#;

#[test]
fn module_file_usize_accum_must_not_add_i32_literals() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("_i32")
        && (rs.contains("pos = pos +") || rs.contains("pos += "));
    assert!(
        !bad,
        "RED MultiFile mixed usize pos with _i32 literals:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
