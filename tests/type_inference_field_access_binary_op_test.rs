/// TDD Test: Float inference for field access in binary operations
///
/// Bug: self.field * literal defaults to f64 even when field is f32
/// Pattern: Field access in binary op doesn't constrain the literal
/// Expected: Literal should match the field type
///
/// Example from breach-protocol:
/// ```windjammer
/// pub struct Grid { pub cell_size: f32 }
/// impl Grid {
///     pub fn get_world_pos(self, x: i32) -> f32 {
///         x as f32 * self.cell_size + self.cell_size * 0.5  // Should be 0.5_f32!
///     }
/// }
/// ```

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_field_access_times_literal() {
    let source = r#"pub struct Grid {
    pub cell_size: f32,
}

impl Grid {
    pub fn get_world_x(self, grid_x: i32) -> f32 {
        grid_x as f32 * self.cell_size + self.cell_size * 0.5
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // The literal 0.5 should be f32 (field type)
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32' (field is f32), got: {}",
        output
    );
    assert!(
        !output.contains("0.5_f64"),
        "Should not contain '0.5_f64': {}",
        output
    );
}

#[test]
fn test_math_constant_with_field() {
    let source = r#"pub struct Entity {
    pub angle: f32,
}

impl Entity {
    pub fn rotate_degrees(self) -> f32 {
        self.angle * 57.29578
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Math constant should match field type
    assert!(
        output.contains("57.29578_f32"),
        "Expected math constant as f32"
    );
}

#[test]
fn test_field_in_complex_expression() {
    let source = r#"pub struct Formation {
    pub spacing: f32,
}

impl Formation {
    pub fn get_offset(self, index: i32) -> f32 {
        index as f32 * self.spacing * 0.5
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32' in chained binary op"
    );
}

// Helper function
fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("field_binary_test_{}_{}", std::process::id(), unique_id);
    let test_file = temp_dir.join(format!("{}.wj", test_name));
    let output_dir = temp_dir.join(&test_name);
    let output_file = output_dir.join(format!("{}.rs", test_name));
    
    std::fs::write(&test_file, source).expect("Failed to write test file");
    
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/wj");
    
    let status = Command::new(&wj_path)
        .arg("build")
        .arg(&test_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj compiler");
    
    assert!(status.success(), "Compilation failed");
    
    let rust_code = std::fs::read_to_string(&output_file)
        .expect("Failed to read generated Rust file");
    
    let _ = std::fs::remove_file(&test_file);
    let _ = std::fs::remove_dir_all(&output_dir);
    
    rust_code
}
