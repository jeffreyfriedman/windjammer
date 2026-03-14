/// TDD: Float inference in comparison with zero (len > 0.0)
///
/// Bug: In physics/advanced_collision.wj get_axes(), `if len > 0.0` infers 0.0 as f64
/// but len is f32 (from Vec2 field operations). Rust rejects f32 > f64.
///
/// Root cause: len = (edge_x * edge_x + edge_y * edge_y).sqrt() - the MethodCall's
/// return type isn't inferred because infer_type_from_expression doesn't handle Binary.
///
/// Solution: Add infer_type_from_expression for Binary (arithmetic) and fallback for
/// primitive methods (sqrt, etc.) to return object type when not in function_signatures.

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("float_physics_{}_{}", std::process::id(), unique_id);
    let test_file = temp_dir.join(format!("{}.wj", test_name));
    let output_dir = temp_dir.join(&test_name);
    let output_file = output_dir.join(format!("{}.rs", test_name));

    std::fs::create_dir_all(&output_dir).ok();
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
fn test_float_inference_in_comparison_with_zero() {
    // Bug: "if len > 0.0" infers 0.0 as f64, but len is f32
    // len = (x*x + y*y).sqrt() - sqrt returns f32 when receiver is f32
    let source = r#"
pub fn normalize(x: f32, y: f32) -> (f32, f32) {
    let len = (x * x + y * y).sqrt()
    if len > 0.0 {
        return (x / len, y / len)
    }
    return (0.0, 0.0)
}
"#;

    let rust_code = compile_and_get_rust(source);

    // 0.0 in "len > 0.0" and "return (0.0, 0.0)" should all be f32 (return type is (f32, f32))
    assert!(
        !rust_code.contains("0.0_f64"),
        "Should NOT use 0.0_f64 when context is f32. Generated:\n{}",
        rust_code
    );
    // Should use f32 for the comparison literal
    assert!(
        rust_code.contains("0.0_f32"),
        "Should use 0.0_f32 in comparison. Generated:\n{}",
        rust_code
    );
}

#[test]
fn test_float_inference_propagates_from_left_operand() {
    // value: f32, so "value > 0.0" should infer 0.0 as f32
    let source = r#"
pub fn check_positive(value: f32) -> bool {
    return value > 0.0
}
"#;

    let rust_code = compile_and_get_rust(source);

    assert!(
        rust_code.contains("0.0_f32"),
        "value > 0.0 should generate 0.0_f32 when value is f32. Generated:\n{}",
        rust_code
    );
}

