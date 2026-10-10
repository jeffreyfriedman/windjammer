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

//! indexed Copy tuple **swap** must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn swap_at(sorted: Vec<(u64, f32)>, i: int, j: int) -> Vec<(u64, f32)> {
    let mut sorted = sorted
    let tmp = sorted[i]
    sorted[i] = sorted[j]
    sorted[j] = tmp
    sorted
}
"#;

#[test]
fn module_file_indexed_copy_tuple_swap_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("sorted[i].clone()") || rs.contains("sorted[j].clone()");
    assert!(
        !bad,
        "RED indexed Copy tuple cloned on swap:\n{rs}"
    );
    let _ = test.cargo_check();
}
