// TDD Test: Float literal inference in EXACT astar_grid pattern
//
// Bug: result.push((x, y, self.get_cost(x, y) * 1.414)) generates f64
// This is the EXACT pattern from astar_grid.wj that fails
//
// Dogfooding Win: Extracted from real game code

use std::fs;
use std::process::Command;

#[test]
fn test_float_literal_in_tuple_push_with_method_return() {
    let wj_source = r#"
struct Grid {
    pub width: i32,
    pub cells: Vec<f32>,
}

impl Grid {
    fn get_cost(self, x: i32, y: i32) -> f32 {
        self.cells[(y * self.width + x) as usize]
    }
    
    fn get_neighbors(self, x: i32, y: i32) -> Vec<(i32, i32, f32)> {
        let mut result = Vec::new()
        result.push((x + 1, y + 1, self.get_cost(x + 1, y + 1) * 1.414))
        result
    }
}
"#;

    let output_dir = "/tmp/wj_test_astar_pattern";
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
    eprintln!("Compiler stderr:\n{}", stderr);

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        stderr
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    eprintln!("Generated Rust:\n{}", rust_code);

    // Check debug output
    if stderr.contains("TDD DEBUG") {
        let debug_lines: Vec<_> = stderr
            .lines()
            .filter(|l| l.contains("TDD DEBUG"))
            .collect();
        eprintln!("All debug output:\n{}", debug_lines.join("\n"));
    }

    // The literal 1.414 in tuple should be f32 (from get_cost: f32)
    assert!(
        !rust_code.contains("1.414_f64"),
        "1.414 should NOT be f64 when multiplying f32 method return in tuple, got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("1.414_f32") || rust_code.contains("1.414f32"),
        "1.414 should be f32 when multiplying f32 method return in tuple, got:\n{}",
        rust_code
    );
}
