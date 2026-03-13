/// TDD: Comprehensive Float Literal Type Inference
///
/// Root cause: Windjammer hardcoded all float literals to f64, causing ~150+ errors
/// when variables/params/fields expect f32.
///
/// Solution: Constraint-based inference propagates expected type from:
/// - Variable declaration: let x: f32 = 1.0
/// - Function parameter: fn foo(x: f32) → foo(1.0)
/// - Struct field: Vec3 { x: 1.0 } where x: f32
/// - Binary operation: f32_var + 2.0
/// - Assignment: f32_field = 0.0

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("float_inf_test_{}_{}", std::process::id(), unique_id);
    let test_file = temp_dir.join(format!("{}.wj", test_name));
    let output_dir = temp_dir.join(&test_name);
    let output_file = output_dir.join(format!("{}.rs", test_name));

    std::fs::write(&test_file, source).expect("Failed to write test file");

    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let status = Command::new(&wj_path)
        .arg("build")
        .arg(&test_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj compiler");

    assert!(status.success(), "Compilation failed");

    let rust_code = std::fs::read_to_string(&output_file).expect("Failed to read generated Rust");

    let _ = std::fs::remove_file(&test_file);
    let _ = std::fs::remove_dir_all(&output_dir);

    rust_code
}

#[test]
fn test_float_literal_infers_from_variable_type() {
    let source = r#"
fn test() {
    let x: f32 = 1.0
    let y: f64 = 2.0
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1_f32"),
        "let x: f32 = 1.0 should generate _f32, got:\n{}",
        output
    );
    assert!(
        output.contains("2.0_f64") || output.contains("2_f64"),
        "let y: f64 = 2.0 should generate _f64, got:\n{}",
        output
    );
}

#[test]
fn test_float_literal_infers_from_function_param() {
    let source = r#"
fn takes_f32(x: f32) { }
fn main() {
    takes_f32(1.0)
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32") || output.contains("takes_f32(1.0_f32)"),
        "takes_f32(1.0) should generate 1.0_f32, got:\n{}",
        output
    );
}

#[test]
fn test_float_literal_infers_from_struct_field() {
    let source = r#"
struct Vec3 { x: f32, y: f32, z: f32 }
fn test() {
    let v = Vec3 { x: 1.0, y: 2.0, z: 3.0 }
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32") && output.contains("2.0_f32") && output.contains("3.0_f32"),
        "Vec3 literals should be f32, got:\n{}",
        output
    );
}

#[test]
fn test_float_binary_ops_preserve_type() {
    let source = r#"
fn test() {
    let x: f32 = 1.0
    let result = x + 2.0
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("2.0_f32"),
        "2.0 in x + 2.0 should infer f32 from x, got:\n{}",
        output
    );
}

#[test]
fn test_float_default_is_f64() {
    let source = r#"
fn test() {
    let x = 1.0
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("_f64"),
        "Unconstrained 1.0 should default to f64, got:\n{}",
        output
    );
}

#[test]
fn test_field_times_literal_infers_f32() {
    let source = r#"pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn scaled(self, factor: f32) -> Point {
        Point {
            x: self.x * 0.5,
            y: self.y * 2.0,
        }
    }
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.5_f32") && output.contains("2.0_f32"),
        "Field * literal should infer f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("_f64"),
        "Should not have f64 in f32 context, got:\n{}",
        output
    );
}
