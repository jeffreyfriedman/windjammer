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

//! i32 coords must not compare against `goal as usize` peers.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn at_goal(current_x: i32, current_y: i32, goal_x: i32, goal_y: i32) -> bool {
    current_x == goal_x && current_y == goal_y
}
"#;

/// i32 field loaded via `vec[usize_idx].x` then compared to i32 goal.
const SRC_INDEXED_FIELD: &str = r#"
pub struct AStarNode {
    pub x: i32,
    pub y: i32,
}

pub fn at_goal_from_open(open_set: Vec<AStarNode>, best_idx_usize: usize, goal_x: i32, goal_y: i32) -> bool {
    let current_x = open_set[best_idx_usize].x
    let current_y = open_set[best_idx_usize].y
    current_x == goal_x && current_y == goal_y
}
"#;

#[test]
fn module_file_i32_coord_compare_must_not_cast_peer_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as usize");
    assert!(
        !bad,
        "RED i32==i32 compare emitted as usize peer:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_indexed_i32_field_must_not_cast_goal_to_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_INDEXED_FIELD);
    let map = test.compile().expect("indexed compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("indexed MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("goal_x as usize")
        || rs.contains("goal_y as usize")
        || rs.contains("current_x as i32")
        || rs.contains("current_y as i32");
    assert!(
        !bad,
        "RED indexed i32 field compare cast goal/current through usize:\n{rs}"
    );
    test.cargo_check().expect("indexed cargo-check");
}
