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

//! Regression: module file usize loop index assign must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Node {
    pub f_score: f32,
}

pub fn find_min(nodes: Vec<Node>) -> i32 {
    if nodes.len() == 0 {
        return -1
    }
    let mut best_idx = 0
    let mut best_f = nodes[0].f_score
    let mut i = 1
    while i < nodes.len() {
        if nodes[i].f_score < best_f {
            best_f = nodes[i].f_score
            best_idx = i
        }
        i = i + 1
    }
    best_idx as i32
}
"#;

#[test]
fn module_file_usize_loop_index_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("i.clone()") || rs.contains("best_idx = i.clone()");
    assert!(
        !bad,
        "RED: Copy loop index cloned on assign:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
