#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Test: Math Constant Type Inference
///
/// Bug: Math constants (PI, degrees conversion, etc.) default to f64
/// Example: angle * 57.295827908797776 (radians→degrees) generates _f64
/// Root Cause: High-precision constants not constrained by context
/// Expected: If used with f32, constant should be f32
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_radians_to_degrees_constant() {
    let source = r#"
pub fn radians_to_degrees(radians: f32) -> f32 {
    radians * 57.295827908797776
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The high-precision constant should be f32 (matches param and return type)
    assert!(
        output.contains("57.295827908797776_f32") || output.contains("57.29582790879778_f32"),
        "Expected '57.295827908797776_f32' (radians to degrees constant), got: {}",
        output
    );
    assert!(
        !output.contains("57.295827908797776_f64"),
        "Should not contain '_f64': {}",
        output
    );
}

#[test]
fn test_degrees_to_radians_constant() {
    let source = r#"
pub fn degrees_to_radians(degrees: f32) -> f32 {
    degrees * 0.017453292519943295
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        output.contains("0.017453292519943295_f32") || output.contains("0.01745329251994329_f32"),
        "Expected '0.017453292519943295_f32' (degrees to radians constant), got: {}",
        output
    );
}

#[test]
fn test_pi_constant() {
    let source = r#"
pub fn circle_circumference(radius: f32) -> f32 {
    2.0 * 3.141592653589793 * radius
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Constant folding: 2.0 * PI = TAU (6.283185307179586)
    // Should be folded to f32 since radius is f32
    assert!(
        (output.contains("6.283185307179586_f32") || output.contains("6.283185307179587_f32"))
            || (output.contains("3.141592653589793_f32") && output.contains("2.0_f32")),
        "Expected PI/TAU constant as f32, got: {}",
        output
    );
}

#[test]
fn test_math_constant_in_struct_field() {
    let source = r#"
struct Physics {
    gravity: f32,
}

impl Physics {
    pub fn new() -> Physics {
        Physics {
            gravity: 9.80665,
        }
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Gravity constant should be f32 (field type)
    assert!(
        output.contains("9.80665_f32"),
        "Expected '9.80665_f32' (gravity constant), got: {}",
        output
    );
}

#[test]
fn test_math_constant_in_method_call() {
    let source = r#"
pub fn clamp_angle(angle: f32) -> f32 {
    angle.min(6.283185307179586).max(0.0)
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // TAU constant (2*PI) should be f32
    assert!(
        output.contains("6.283185307179586_f32") || output.contains("6.283185307179587_f32"),
        "Expected TAU constant as f32, got: {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
