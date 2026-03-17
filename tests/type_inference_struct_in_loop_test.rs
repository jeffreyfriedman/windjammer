// TDD Test: Float literal in struct pushed to Vec in loop
//
// Bug: cells.push(Cell { cost: 1.0 }) generates 1.0_f64 in while loop
// Pattern from astar_grid.wj that's failing
//
// Dogfooding Win: This exact pattern fails in game code

use std::fs;
use std::process::Command;

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

    let output_dir = "/tmp/wj_test_struct_loop";
    fs::create_dir_all(output_dir).unwrap();
    fs::write(format!("{}/test.wj", output_dir), wj_source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args([
            "build",
            "--target",
            "rust",
            "--no-cargo",
            &format!("{}/test.wj", output_dir),
            "--output",
            output_dir,
        ])
        .current_dir("/Users/jeffreyfriedman/src/wj/windjammer")
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // TDD DEBUG: Always print stderr to see debug output
    eprintln!("Compiler stderr:\n{}", stderr);

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        stderr
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 1.0 in cells.push(Cell { cost: 1.0 }) should be f32
    assert!(
        !rust_code.contains("cost: 1.0_f64") && !rust_code.contains("cost: 1_f64"),
        "1.0 should NOT be f64 when assigned to f32 struct field in Vec::push, got:\n{}",
        rust_code
    );
}
