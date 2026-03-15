/// TDD: Float inference for chained binary operations (f32/f64 mixing fix)
///
/// BUG: Constants like 6.28318 (2*PI) are inferred as f64, but used with f32 values.
/// Pattern: `x as f32 * 6.28318 / count as f32` → 6.28318 becomes f64, causing E0277.
///
/// ROOT CAUSE: get_known_float_type_from_expr doesn't handle Cast, and we only do LHS→RHS
/// (not RHS→LHS). So `6.28318 / count as f32` - the literal 6.28318 never gets f32.
///
/// FIX: Approach A - Bidirectional propagation in chains:
/// - Add Cast to get_known_float_type_from_expr (x as f32 → Some(F32))
/// - Add RHS→LHS when RHS has known type and LHS is literal
/// - Result: If ANY operand in chain is f32, all literals → f32
///
/// Test cases from game: squad_tactics, particle_emitter3d, emitter, etc.

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("float_chained_{}_{}", std::process::id(), unique_id);
    let test_file = temp_dir.join(format!("{}.wj", test_name));
    let output_dir = temp_dir.join(&test_name);
    let output_file = output_dir.join(format!("{}.rs", test_name));

    std::fs::create_dir_all(&output_dir).expect("Failed to create output dir");
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

// =============================================================================
// Case 1: x * 2.0 / y where x, y are f32
// =============================================================================

#[test]
fn test_chained_mul_div_f32_params() {
    let source = r#"
pub fn area(x: f32, y: f32) -> f32 {
    x * 2.0 / y
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("2.0_f32"),
        "x * 2.0 / y where x,y are f32 should generate 2.0_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("2.0_f64"),
        "Should not generate 2.0_f64, got:\n{}",
        output
    );
}

// =============================================================================
// Case 2: (x * 0.1).sin() * 0.5 where x is f32
// =============================================================================

#[test]
fn test_method_result_times_literal_sin() {
    let source = r#"
pub fn wave(t: f32) -> f32 {
    (t * 0.1).sin() * 0.5
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.1_f32") && output.contains("0.5_f32"),
        "(t * 0.1).sin() * 0.5 should generate both literals as f32, got:\n{}",
        output
    );
}

// =============================================================================
// Case 3: member_index as f32 * 6.28318 / count as f32 (squad_tactics pattern)
// =============================================================================

#[test]
fn test_cast_times_pi_over_cast() {
    let source = r#"
pub fn compute_angle(member_index: i32, count: i32) -> f32 {
    let angle = (member_index as f32) * (6.28318 / count as f32)
    angle
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("6.28318_f32"),
        "6.28318 in f32 context should be 6.28318_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("6.28318_f64"),
        "Should not generate 6.28318_f64, got:\n{}",
        output
    );
}

// =============================================================================
// Case 4: (seed as f32 * 0.1).sin() * 0.5 + 0.5 (particle_emitter3d pattern)
// =============================================================================

#[test]
fn test_cast_mul_sin_mul_add() {
    let source = r#"
pub fn sample(seed: i32) -> f32 {
    let s = (seed as f32 * 0.1).sin() * 0.5 + 0.5
    s
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.1_f32") && output.contains("0.5_f32"),
        "All literals in (seed as f32 * 0.1).sin() * 0.5 + 0.5 should be f32, got:\n{}",
        output
    );
}

// =============================================================================
// Case 5: s * 6.28318 where s is f32 (from previous computation)
// =============================================================================

#[test]
fn test_f32_var_times_pi() {
    let source = r#"
pub fn to_radians(s: f32) -> f32 {
    s * 6.28318
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("6.28318_f32"),
        "s * 6.28318 where s: f32 should generate 6.28318_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Case 6: RHS→LHS: 6.28318 / count as f32 (literal on LHS, typed RHS)
// =============================================================================

#[test]
fn test_literal_over_cast_rhs_to_lhs() {
    let source = r#"
pub fn tau_over_count(count: i32) -> f32 {
    6.28318 / count as f32
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("6.28318_f32"),
        "6.28318 / count as f32 should propagate f32 from RHS to literal, got:\n{}",
        output
    );
}

// =============================================================================
// Case 7: seed as f32 * 12.9898 (particle rand_offset pattern)
// =============================================================================

#[test]
fn test_cast_times_literal() {
    let source = r#"
pub fn rand_seed(seed: i32) -> f32 {
    (seed as f32 * 12.9898).sin()
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("12.9898_f32"),
        "seed as f32 * 12.9898 should generate 12.9898_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Case 8: width * height * 0.5 (chained with two f32 params)
// =============================================================================

#[test]
fn test_chained_three_operands() {
    let source = r#"
pub fn triangle_area(width: f32, height: f32) -> f32 {
    width * height * 0.5
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.5_f32"),
        "width * height * 0.5 should generate 0.5_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Case 9: s * 6.28318 where s = (seed as f32 * 0.1).sin() * 0.5 + 0.5 (particle_emitter3d)
// =============================================================================
// BUG: s is inferred from complex expression - var_types must get s → f32 for s * 6.28318
// ROOT CAUSE: infer_type_from_expression didn't handle Cast or Literal, so s never got f32

#[test]
fn test_f32_var_from_complex_expr_times_pi() {
    let source = r#"
pub fn sphere_point(seed: i32, radius: f32) -> f32 {
    let s = (seed as f32 * 0.1).sin() * 0.5 + 0.5
    let x = (s * 6.28318).cos() * radius
    x
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("6.28318_f32"),
        "s * 6.28318 where s from f32 chain should generate 6.28318_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("6.28318_f64"),
        "Should not generate 6.28318_f64, got:\n{}",
        output
    );
    assert!(
        output.contains("0.1_f32") && output.contains("0.5_f32"),
        "All literals in s chain should be f32, got:\n{}",
        output
    );
}

// Case 10: (1.0 - t * t).sqrt() * radius - nested binary with f32
// =============================================================================

#[test]
fn test_nested_binary_sqrt_times_radius() {
    let source = r#"
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 { Vec3 { x, y, z } }
}

pub fn sphere_point(t: f32, radius: f32) -> Vec3 {
    let x = (1.0 - t * t).sqrt() * radius
    Vec3::new(x, 0.0, 0.0)
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32") && output.contains("radius"),
        "Literals in (1.0 - t * t).sqrt() * radius should be f32, got:\n{}",
        output
    );
}
