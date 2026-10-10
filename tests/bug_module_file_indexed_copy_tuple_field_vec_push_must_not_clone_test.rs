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

//! DB-466: indexed Copy tuple **field** into `Vec::push` must not `.clone()`.
//!
//! Product `scene_graph/scene_graph_state.rs` `sort_by_distance`:
//!   `sorted_nodes.push(sorted[k].0.clone())` and `sorted[i].1.clone()`
//! WJ uses bare `sorted[k].0` / `sorted[i].1` (`u64` / `f32`, Copy).
//! Distinct from DB-439 (tuple field **return**), DB-465 (clone the whole
//! indexed tuple into push), DB-363 (clone the element then read a field).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn nearer(sorted: Vec<(u64, f32)>, i: int, j: int) -> bool {
    let a_dist = sorted[i].1
    let b_dist = sorted[j].1
    a_dist > b_dist
}

pub fn ids_in_order(sorted: Vec<(u64, f32)>) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new()
    let mut k: int = 0
    while k < sorted.len() {
        out.push(sorted[k].0)
        k = k + 1
    }
    out
}
"#;

#[test]
fn module_file_indexed_copy_tuple_field_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".0.clone()") || rs.contains(".1.clone()");
    assert!(
        !bad,
        "indexed Copy tuple field cloned:\n{rs}"
    );
    let _ = test.cargo_check();
}
