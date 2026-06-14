#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! Copy tuple from `Vec<(…)>[i]`: Rust yields the tuple by value; no explicit `*` on the index.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_tuple_from_copy_vec_index_no_star() {
    let source = r#"
use std::collections::HashMap

pub fn pathfind() -> i32 {
    let mut g_score: HashMap<(i32, i32), f32> = HashMap::new()
    g_score.insert((0, 0), 0.0)
    let neighbors: Vec<(i32, i32, f32)> = Vec::new()
    let mut ni = 0
    while ni < neighbors.len() {
        let (nx, ny, move_cost) = neighbors[ni]
        let _ = g_score.get(&(nx, ny))
        ni = ni + 1
    }
    0
}
"#;

    let rust = test_utils::compile_single(source);

    assert!(
        !rust.contains("*neighbors[ni]") && !rust.contains("* neighbors[ni]"),
        "must not deref Copy tuple index (E0614). got:\n{}",
        rust
    );
}
