//! TDD Test: Methods using self.field in binary ops should be &self, not self
//!
//! When a method reads self.field in arithmetic expressions like `x + self.offset`,
//! the method should be inferred as `&self` (borrowed), not `self` (owned).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_get_generated(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return (false, String::new(), stderr);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    (true, generated, String::new())
}

#[test]
fn test_self_field_in_binary_op_should_borrow() {
    // Use a non-Copy type (has String field) to avoid Copy type special casing
    let code = r#"
pub struct Editor {
    name: string,
    offset_x: f32,
    offset_y: f32,
}

impl Editor {
    pub fn translate_x(self, x: f32) -> f32 {
        x + self.offset_x
    }
    
    pub fn translate_point(self, x: f32, y: f32) -> (f32, f32) {
        (x + self.offset_x, y + self.offset_y)
    }
}
"#;

    let (success, generated, err) = compile_and_get_generated(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Compile error:\n{}", err);
    }

    assert!(success, "Compilation should succeed");

    // Methods should have &self, not self
    assert!(
        generated.contains("fn translate_x(&self"),
        "translate_x should be &self. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn translate_point(&self"),
        "translate_point should be &self. Generated:\n{}",
        generated
    );
}
