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

//! `u32` index into `[T]` / `Vec` must cast to `usize` (not index by `u32`).
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn degree_at(offsets: Vec<u32>, i: u32) -> u32 {
    offsets[i + 1] - offsets[i]
}
"#;

#[test]
fn module_file_u32_index_into_vec_must_cast_to_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    // Bad: offsets[i + 1] or offsets[i] when i is u32 without as usize
    let has_u32_i = rs.contains("i: u32") || rs.contains("i:u32");
    let bad = has_u32_i
        && (rs.contains("offsets[i + 1]")
            || rs.contains("offsets[i+1]")
            || (rs.contains("offsets[i]") && !rs.contains("offsets[i as usize]")));
    assert!(
        !bad,
        "RED MultiFile indexes Vec with bare u32 (need as usize):\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
