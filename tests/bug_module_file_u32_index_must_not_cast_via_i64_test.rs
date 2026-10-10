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

//! u32 index into `[]` must not emit `as i64 as usize`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn bump(bins: Vec<u32>, clamped: u32) {
    bins[clamped as usize] = bins[clamped as usize] + 1
}
"#;

const SRC_VIA_I64: &str = r#"
pub fn bump(bins: Vec<u32>, clamped: u32) {
    bins[clamped as i64] = bins[clamped as i64] + 1
}
"#;

#[test]
fn module_file_u32_index_must_not_cast_via_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as i64 as usize") || rs.contains("as i32 as usize");
    assert!(
        !bad,
        "RED u32 index double-cast via i64/i32:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_index_as_i64_must_fold_to_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_VIA_I64);
    let map = test.compile().expect("via-i64 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("via-i64 lib.rs:\n{rs}");
    let bad = rs.contains("as i64 as usize");
    assert!(
        !bad,
        "RED index as i64 as usize not folded:\n{rs}"
    );
    assert!(
        rs.contains("as usize") || rs.contains("bins[clamped]"),
        "expected usize index. Got:\n{rs}"
    );
    test.cargo_check().expect("via-i64 cargo-check");
}
