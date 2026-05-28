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

// TDD Test: Float literal in struct pushed to Vec in loop
//
// Bug: cells.push(Cell { cost: 1.0 }) generates 1.0_f64 in while loop
// Pattern from astar_grid.wj that's failing
//
// Dogfooding Win: This exact pattern fails in game code

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_struct_literal_in_vec_push_loop() {
    let wj_source = r#"
pub struct Cell {
    pub walkable: bool,
    pub cost: f32,
}

pub fn new_grid(size: i32) -> Vec<Cell> {
    let mut cells = Vec::new()
    let mut i = 0
    while i < size {
        cells.push(Cell { walkable: true, cost: 1.0 })
        i = i + 1
    }
    cells
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 1.0 in cells.push(Cell { cost: 1.0 }) should be f32
    assert!(
        !rust_code.contains("cost: 1.0_f64") && !rust_code.contains("cost: 1_f64"),
        "1.0 should NOT be f64 when assigned to f32 struct field in Vec::push, got:\n{}",
        rust_code
    );
}
