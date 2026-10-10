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

//! DB-418: early-return of an owned `Vec` formal must move, not `positions.clone()`.
//!
//! Product `uv_unwrap_algorithm.rs` / `uv_island_packing.rs`:
//!   `return empty_result(positions.clone())` after `positions.len()` (and later
//!   sibling-path uses of `positions`).
//! WJ is `return empty_result(positions)` — that path is exclusive last-use.
//! Distinct from DB-415 (if-then move, no early return + later sibling uses),
//! DB-417 (match-arm `chunk.clone()`), and DB-407 (`Vec` `new` demote).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn empty_result(positions: Vec<i32>) -> i32 {
    positions.len() as i32
}

pub fn unwrap_planar(positions: Vec<i32>, indices: Vec<i32>) -> i32 {
    if positions.len() == 0 || indices.len() < 3 {
        return empty_result(positions)
    }
    let mut i = 0
    while i < indices.len() {
        let vi = indices[i]
        if vi >= positions.len() as i32 {
            return empty_result(positions)
        }
        let p = positions[vi as usize]
        i = i + 1 + p * 0
    }
    positions.len() as i32
}
"#;

#[test]
fn module_file_early_return_must_move_owned_vec() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("positions.clone()");
    assert!(
        !cloned,
        "early-return cloned owned Vec formal:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
