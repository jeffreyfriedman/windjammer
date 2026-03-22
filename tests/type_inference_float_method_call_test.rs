// TDD Test: Float literal inference with method call returning f32
//
// Bug: self.get_cost() * 1.414 generates 1.414_f64 instead of 1.414_f32
// Expected: Binary op with f32 method return should constrain literal to f32
//
// Dogfooding Win: This is a real bug found in astar_grid.wj

use std::fs;
use std::process::Command;

#[test]
fn test_float_literal_in_binary_op_with_method_return() {
    let wj_source = r#"
struct Grid {
    pub cost: f32,
}

impl Grid {
    fn get_cost(self) -> f32 {
        self.cost
    }
    
    fn scaled_cost(self) -> f32 {
        self.get_cost() * 1.414
    }
}
"#;

    let output_dir = "/tmp/wj_test_float_method";
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
    eprintln!("Compiler stderr:\n{}", stderr);

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        stderr
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    eprintln!("Generated Rust:\n{}", rust_code);

    // Check debug output for inference result
    if stderr.contains("TDD DEBUG: generate_literal_with_context(1.414)") {
        let debug_lines: Vec<_> = stderr
            .lines()
            .filter(|l| l.contains("generate_literal_with_context(1.414)"))
            .collect();
        eprintln!("Debug output for 1.414:\n{}", debug_lines.join("\n"));
        
        assert!(
            debug_lines.iter().any(|l| l.contains("F32")),
            "Inference should return F32 for 1.414 in binary op with f32 method return"
        );
    }

    // The literal 1.414 in `self.get_cost() * 1.414` should be f32 (from get_cost: f32)
    assert!(
        !rust_code.contains("1.414_f64"),
        "1.414 should NOT be f64 when multiplying f32 method return, got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("1.414_f32") || rust_code.contains("1.414f32"),
        "1.414 should be f32 when multiplying f32 method return, got:\n{}",
        rust_code
    );
}
