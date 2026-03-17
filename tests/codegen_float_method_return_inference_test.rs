/// TDD: Float literal inference with method return types
/// BUG: `self.get_cost() * 1.414` generates f64 instead of f32
/// FIX: Use float inference results in binary operations

use std::fs;
use std::process::Command;

#[test]
fn test_float_literal_with_f32_method_return() {
    let wj_source = r#"
struct Grid {
    pub cost: f32,
}

impl Grid {
    fn get_cost(self) -> f32 {
        self.cost
    }
    
    fn scaled_cost(self) -> f32 {
        self.get_cost() * 1.414  // Should be 1.414_f32
    }
}

fn main() {
    let g = Grid { cost: 10.0 }
    let c = g.scaled_cost()
    println!("{}", c)
}
"#;

    let output_dir = "/tmp/wj_test_method_f32";
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

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    println!("Generated Rust code:\n{}", rust_code);

    // The literal 1.414 should be f32 (get_cost returns f32)
    assert!(
        !rust_code.contains("1.414_f64"),
        "1.414 should NOT be f64 when multiplying f32 method return, got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("1.414_f32") || !rust_code.contains("_f64"),
        "1.414 should be f32 or have no f64 suffixes, got:\n{}",
        rust_code
    );
}
