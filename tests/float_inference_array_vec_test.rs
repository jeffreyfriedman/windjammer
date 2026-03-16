/// TDD: Float literal inference for array and Vec literals
///
/// Problem: vec![1.0, 2.0, 3.0] where Vec<f32> generates f64 elements, causing E0308 errors.
///
/// Goal: When float literals appear in array/Vec literals, infer from the container's element type.
///
/// Architecture:
/// - Default for vec![1.0] (no type annotation) remains f64
/// - Type annotations on left side: let v: Vec<f32> = vec![1.0, 2.0, 3.0]
/// - Only infer when element type is explicitly specified or constrained

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("float_arr_vec_{}_{}", std::process::id(), unique_id);
    let test_file = temp_dir.join(format!("{}.wj", test_name));
    let output_dir = temp_dir.join(&test_name);
    let output_file = output_dir.join(format!("{}.rs", test_name));

    std::fs::create_dir_all(&output_dir).unwrap();
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
fn test_vec_macro_with_f32_annotation() {
    // let v: Vec<f32> = vec![1.0, 2.0, 3.0] → literals should be f32
    let source = r#"
fn test() {
    let v: Vec<f32> = vec![1.0, 2.0, 3.0]
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32") && output.contains("2.0_f32") && output.contains("3.0_f32"),
        "Vec<f32> = vec![1.0, 2.0, 3.0] should generate _f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64") && !output.contains("2.0_f64") && !output.contains("3.0_f64"),
        "Should not have f64 when Vec<f32> is annotated, got:\n{}",
        output
    );
}

#[test]
fn test_array_literal_with_f32_annotation() {
    // let a: [f32; 3] = [1.0, 2.0, 3.0] → literals should be f32
    let source = r#"
fn test() {
    let a: [f32; 3] = [1.0, 2.0, 3.0]
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32") && output.contains("2.0_f32") && output.contains("3.0_f32"),
        "[f32; 3] = [1.0, 2.0, 3.0] should generate _f32, got:\n{}",
        output
    );
}

#[test]
fn test_array_repeat_with_f32_annotation() {
    // let a: [f32; 10] = [1.0; 10] → literal should be f32
    let source = r#"
fn test() {
    let a: [f32; 10] = [1.0; 10]
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32"),
        "[f32; 10] = [1.0; 10] should generate 1.0_f32, got:\n{}",
        output
    );
}

#[test]
fn test_nested_vec_vec_f32() {
    // vec![vec![1.0, 2.0], vec![3.0, 4.0]] with Vec<Vec<f32>>
    let source = r#"
fn test() {
    let v: Vec<Vec<f32>> = vec![vec![1.0, 2.0], vec![3.0, 4.0]]
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32") && output.contains("2.0_f32") && output.contains("3.0_f32") && output.contains("4.0_f32"),
        "Vec<Vec<f32>> nested vec! should generate _f32 for all literals, got:\n{}",
        output
    );
}

#[test]
fn test_no_annotation_defaults_f32() {
    // vec![1.0, 2.0] without type annotation → defaults to f32 (game engine standard)
    let source = r#"
fn test() {
    let v = vec![1.0, 2.0]
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("_f32") || output.contains("1.0_f32") || output.contains("2.0_f32"),
        "Unannotated vec![1.0, 2.0] should default to f32, got:\n{}",
        output
    );
}

#[test]
fn test_vec_f64_explicit() {
    // let v: Vec<f64> = vec![1.0, 2.0, 3.0] → literals should be f64
    let source = r#"
fn test() {
    let v: Vec<f64> = vec![1.0, 2.0, 3.0]
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f64") && output.contains("2.0_f64") && output.contains("3.0_f64"),
        "Vec<f64> = vec![1.0, 2.0, 3.0] should generate _f64, got:\n{}",
        output
    );
}

#[test]
fn test_array_f64_annotation() {
    // let a: [f64; 3] = [1.0, 2.0, 3.0] → literals should be f64
    let source = r#"
fn test() {
    let a: [f64; 3] = [1.0, 2.0, 3.0]
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f64") && output.contains("2.0_f64") && output.contains("3.0_f64"),
        "[f64; 3] = [1.0, 2.0, 3.0] should generate _f64, got:\n{}",
        output
    );
}

#[test]
fn test_vec_repeat_with_f32() {
    // vec![1.0; 5] with Vec<f32>
    let source = r#"
fn test() {
    let v: Vec<f32> = vec![1.0; 5]
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32"),
        "Vec<f32> = vec![1.0; 5] should generate 1.0_f32, got:\n{}",
        output
    );
}
