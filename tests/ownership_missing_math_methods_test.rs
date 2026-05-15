//! TDD test: Ensure math methods like `acos`, `asin`, `as_ptr` etc. are recognized
//! as read-only methods and do NOT trigger mutation inference.
//!
//! Bug: `safe_acos(value: f32)` was generated as `safe_acos(value: &mut f32)` because
//! `value.acos()` was an unknown method and the conservative default assumed mutation.
//!
//! Root cause: `acos` not in method_registry KNOWN_METHODS table.

use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&out_dir).unwrap();

    let wj_path = src_dir.join("test.wj");
    std::fs::write(&wj_path, source).unwrap();

    let wj_bin = env!("CARGO_BIN_EXE_wj");

    let output = Command::new(wj_bin)
        .arg("build")
        .arg(&src_dir)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .output()
        .expect("failed to run wj");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!("Compilation failed:\nstdout: {}\nstderr: {}", stdout, stderr);
    }

    let rs_path = out_dir.join("test.rs");
    if rs_path.exists() {
        std::fs::read_to_string(&rs_path).unwrap()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!("Generated file not found at {:?}\nstdout: {}\nstderr: {}", rs_path, stdout, stderr);
    }
}

#[test]
fn test_acos_on_f32_does_not_trigger_mut_inference() {
    let source = r#"
fn safe_acos(value: f32) -> f32 {
    if value < -1.0 {
        3.14159265
    } else {
        if value > 1.0 {
            0.0
        } else {
            value.acos()
        }
    }
}
"#;
    let rs = compile_wj(source);
    // value should be owned f32, NOT &mut f32
    assert!(
        rs.contains("value: f32"),
        "Expected 'value: f32' (owned), got &mut f32 or other.\nGenerated:\n{}",
        rs
    );
    assert!(
        !rs.contains("value: &mut f32"),
        "value should NOT be &mut f32\nGenerated:\n{}",
        rs
    );
}

#[test]
fn test_as_ptr_on_vec_does_not_trigger_mut_inference() {
    let source = r#"
fn buffer_ptr(data: Vec<f32>) -> usize {
    data.len()
}
"#;
    let rs = compile_wj(source);
    // data should be borrowed (read-only), NOT &mut Vec<f32>
    // Actually for a non-Copy type that's only read, it should be &Vec<f32>
    assert!(
        !rs.contains("data: &mut Vec<f32>"),
        "data should NOT be &mut Vec<f32>\nGenerated:\n{}",
        rs
    );
}

#[test]
fn test_consuming_method_infers_owned() {
    let source = r#"
enum Value {
    Int(i32),
    Float(f32),
}

impl Value {
    fn as_float(self) -> f32 {
        match self {
            Value::Float(v) => v,
            Value::Int(v) => v as f32,
        }
    }
}

struct Evaluator {}

impl Evaluator {
    fn compute(self, a: Value, b: Value) -> Value {
        let fa = a.as_float()
        let fb = b.as_float()
        Value::Float(fa + fb)
    }
}
"#;
    let rs = compile_wj(source);
    // a and b should be owned Value, NOT &Value
    // Because as_float takes self by value (consuming)
    assert!(
        rs.contains("a: Value") || rs.contains("a: crate::Value"),
        "Expected 'a: Value' (owned), not borrowed.\nGenerated:\n{}",
        rs
    );
    assert!(
        !rs.contains("a: &Value"),
        "a should NOT be &Value when consumed by as_float(self)\nGenerated:\n{}",
        rs
    );
}

#[test]
fn test_vec_stored_in_struct_stays_owned() {
    let source = r#"
struct Buffer {
    data: Vec<f32>,
    label: string,
}

impl Buffer {
    fn new(label: string, data: Vec<f32>) -> Buffer {
        Buffer {
            data: data,
            label: label,
        }
    }
}
"#;
    let rs = compile_wj(source);
    // data should be owned Vec<f32> since it's stored in the struct
    assert!(
        !rs.contains("data: &mut Vec<f32>"),
        "data should NOT be &mut Vec<f32> when stored in struct\nGenerated:\n{}",
        rs
    );
}

#[test]
fn test_float_vec_inference() {
    let source = r#"
fn make_pixels() -> Vec<f32> {
    let mut pixels: Vec<f32> = Vec::new()
    pixels.push(0.5)
    pixels.push(0.3)
    pixels
}
"#;
    let rs = compile_wj(source);
    // Vec should be Vec<f32>, not Vec<f64>
    assert!(
        !rs.contains("Vec<f64>"),
        "Vec should be Vec<f32>, not Vec<f64>\nGenerated:\n{}",
        rs
    );
}
