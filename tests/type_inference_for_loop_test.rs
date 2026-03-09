// TDD Test: Float literal inference in for loops
//
// Bug: For loops not traversed by float inference, just like While loops
// Expected: cells.push(Cell { cost: 1.0 }) in for loop should generate 1.0_f32
//
// Dogfooding Win: Common game pattern

use std::fs;
use std::process::Command;

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

    let output_dir = "/tmp/wj_test_for_loop";
    fs::create_dir_all(output_dir).unwrap();
    fs::write(format!("{}/test.wj", output_dir), wj_source).unwrap();

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--",
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

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        stderr
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 1.0 in for loop should be f32
    assert!(
        !rust_code.contains("cost: 1.0_f64") && !rust_code.contains("cost: 1_f64"),
        "1.0 should NOT be f64 when assigned to f32 struct field in for loop, got:\n{}",
        rust_code
    );
}
