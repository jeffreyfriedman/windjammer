// TDD Test: Float literal inference in for loops
//
// Bug: For loops not traversed by float inference, just like While loops
// Expected: cells.push(Cell { cost: 1.0 }) in for loop should generate 1.0_f32
//
// Dogfooding Win: Common game pattern

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_struct_literal_in_for_loop() {
    let wj_source = r#"
pub struct Cell {
    pub walkable: bool,
    pub cost: f32,
}

pub fn new_grid(size: i32) -> Vec<Cell> {
    let mut cells = Vec::new()
    for i in 0..size {
        cells.push(Cell { walkable: true, cost: 1.0 })
    }
    cells
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 1.0 in for loop should be f32
    assert!(
        !rust_code.contains("cost: 1.0_f64") && !rust_code.contains("cost: 1_f64"),
        "1.0 should NOT be f64 when assigned to f32 struct field in for loop, got:\n{}",
        rust_code
    );
}
