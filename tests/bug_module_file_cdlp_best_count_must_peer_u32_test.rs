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

//!
//!   WJ: `let mut best_count = 0` + `count` from `graph_vertex_u32_get` → u32
//!   compiler: `let mut best_count = 0_i64` then `count > best_count` → E0277/E0308 u32 vs i64.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn majority_label(neighbor_labels: Vec<i64>, counts: Vec<u32>) -> i64 {
    let mut best_label = neighbor_labels[0]
    let mut best_count = 0
    let mut i = 0
    while i < neighbor_labels.len() {
        let label = neighbor_labels[i]
        let count = counts[i]
        if count > best_count || (count == best_count && label < best_label) {
            best_count = count
            best_label = label
        }
        i = i + 1
    }
    best_label
}
"#;

#[test]
fn module_file_u32_count_accum_must_not_init_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("best_count = 0_i64")
        || (rs.contains("0_i64") && rs.contains("best_count"));
    assert!(
        !bad,
        "RED: MultiFile initialized best_count as i64 against u32 count:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

