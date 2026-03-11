/// TDD Test: Compound Expression Type Inference
///
/// Bug: Literals in compound expressions not constrained by other operands
/// Pattern: a * b + b * 0.5 where b is f32, but 0.5 generates as f64
/// Root Cause: Binary ops in compound expressions not propagating type constraints
/// Expected: All literals should match the typed operands

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_field_in_compound_expression() {
    let source = r#"
struct Grid {
    cell_size: f32,
}

impl Grid {
    pub fn grid_to_world(grid_x: i32) -> f32 {
        grid_x as f32 * self.cell_size + self.cell_size * 0.5
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // 0.5 should be f32 because cell_size is f32
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32', got: {}",
        output
    );
    assert!(
        !output.contains("0.5_f64"),
        "Should not contain '0.5_f64': {}",
        output
    );
}

#[test]
fn test_cast_plus_field_times_literal() {
    let source = r#"
struct Camera {
    width: u32,
    zoom: f32,
    position_x: f32,
}

impl Camera {
    pub fn screen_to_world(screen_x: f32) -> f32 {
        (screen_x - self.width as f32 * 0.5) / self.zoom + self.position_x
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // 0.5 should be f32 (used with width as f32)
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32', got: {}",
        output
    );
}

#[test]
fn test_math_constant_in_compound_expression() {
    let source = r#"
pub fn calculate_angle(member_index: i32, total: usize) -> f32 {
    member_index as f32 * 6.28318 / total as f32
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // 6.28318 (TAU) should be f32 (return type is f32, operands are f32)
    assert!(
        output.contains("6.28318_f32"),
        "Expected '6.28318_f32', got: {}",
        output
    );
}

#[test]
fn test_intensity_scaling() {
    let source = r#"
struct Light {
    intensity: f32,
}

impl Light {
    pub fn get_scaled_intensity(elev_factor: f32) -> f32 {
        self.intensity * (0.2 + 0.8 * elev_factor)
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Both 0.2 and 0.8 should be f32
    assert!(
        output.contains("0.2_f32") && output.contains("0.8_f32"),
        "Expected both literals as f32, got: {}",
        output
    );
}

#[test]
fn test_position_offset_calculation() {
    let source = r#"
struct Formation {
    spacing: f32,
}

impl Formation {
    pub fn get_offset(index: i32) -> f32 {
        (index * 73 % 10 - 5) as f32 * self.spacing * 0.5
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // 0.5 should be f32 (spacing is f32)
    assert!(
        output.contains("0.5_f32"),
        "Expected '0.5_f32', got: {}",
        output
    );
}

#[test]
fn test_prediction_calculation() {
    let source = r#"
pub fn predict_position(pos: f32, vel: f32, time: f32) -> f32 {
    pos + vel * time
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // No literals here, but verify it compiles and types are consistent
    assert!(
        !output.contains("_f64"),
        "Should not contain any '_f64': {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
fn compile_and_get_rust(source: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/compound_expr_test_{}_{}", std::process::id(), counter);
    
    std::fs::create_dir_all(&test_dir).unwrap();
    
    let source_file = PathBuf::from(&test_dir).join("test.wj");
    std::fs::write(&source_file, source).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            source_file.to_str().unwrap(),
            "--target", "rust",
            "--output", &test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");
    
    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    let rust_file = PathBuf::from(&test_dir).join("test.rs");
    std::fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file")
}
