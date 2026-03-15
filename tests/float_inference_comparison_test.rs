/// TDD: Float literal inference for comparison operators
///
/// Bug: `x < 2.0` where `x: f32` generates `2.0_f64`, causing "can't compare f32 with f64".
/// Root cause: Float literal inference didn't apply to comparison operators.
///
/// Fix: Extend collect_float_literal_constraints to handle comparison ops (<, >, <=, >=, ==, !=)
/// with LHS→RHS and RHS→LHS propagation (same as arithmetic ops).

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("float_compare_{}_{}", std::process::id(), unique_id);
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
// Basic: f32_var < literal
// =============================================================================

#[test]
fn test_f32_var_lt_literal() {
    let source = r#"
pub fn check(x: f32) -> bool {
    x < 2.0
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("2.0_f32"),
        "x < 2.0 where x: f32 should generate 2.0_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("2.0_f64"),
        "Should NOT use 2.0_f64 when LHS is f32, got:\n{}",
        output
    );
}

// =============================================================================
// Basic: f64_var > literal
// =============================================================================

#[test]
fn test_f64_var_gt_literal() {
    let source = r#"
pub fn check(x: f64) -> bool {
    x > 1.0
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f64"),
        "x > 1.0 where x: f64 should generate 1.0_f64, got:\n{}",
        output
    );
}

// =============================================================================
// Both literals: if x < threshold (threshold from param)
// =============================================================================

#[test]
fn test_both_operands_typed() {
    let source = r#"
pub fn in_range(value: f32, min: f32, max: f32) -> bool {
    min < value && value < max
}
"#;
    let output = compile_and_get_rust(source);
    // value is f32, so comparisons should use f32
    assert!(
        !output.contains("_f64"),
        "All operands f32 - should not use f64, got:\n{}",
        output
    );
}

// =============================================================================
// Chained: min < value && value < max
// =============================================================================

#[test]
fn test_chained_comparisons() {
    let source = r#"
pub fn clamp_check(val: f32, lo: f32, hi: f32) -> bool {
    lo < val && val < hi
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains("_f64"),
        "Chained comparisons with f32 should not use f64, got:\n{}",
        output
    );
}

// =============================================================================
// Method result: (x*x + y*y).sqrt() > 0.0
// =============================================================================

#[test]
fn test_method_result_comparison() {
    let source = r#"
pub fn normalize(x: f32, y: f32) -> (f32, f32) {
    let len = (x * x + y * y).sqrt()
    if len > 0.0 {
        return (x / len, y / len)
    }
    return (0.0, 0.0)
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.0_f32"),
        "len > 0.0 where len is f32 should use 0.0_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Should NOT use 0.0_f64 in f32 context, got:\n{}",
        output
    );
}

// =============================================================================
// Equality: f32_var == 0.0
// =============================================================================

#[test]
fn test_f32_eq_literal() {
    let source = r#"
pub fn is_zero(x: f32) -> bool {
    x == 0.0
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.0_f32"),
        "x == 0.0 where x: f32 should generate 0.0_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Inequality: f32_var != 1.0
// =============================================================================

#[test]
fn test_f32_ne_literal() {
    let source = r#"
pub fn not_one(x: f32) -> bool {
    x != 1.0
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32"),
        "x != 1.0 where x: f32 should generate 1.0_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Less-or-equal, greater-or-equal
// =============================================================================

#[test]
fn test_f32_le_ge_literal() {
    let source = r#"
pub fn in_bounds(x: f32, low: f32, high: f32) -> bool {
    low <= x && x <= high
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        !output.contains("_f64"),
        "All f32 - should not use f64, got:\n{}",
        output
    );
}

// =============================================================================
// Literal on LHS: 0.0 < x where x: f32
// =============================================================================

#[test]
fn test_literal_lhs_f32_rhs() {
    let source = r#"
pub fn is_positive(x: f32) -> bool {
    0.0 < x
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.0_f32"),
        "0.0 < x where x: f32 should infer 0.0 as f32 (RHS→LHS), got:\n{}",
        output
    );
}

// =============================================================================
// Field access: self.x < 1.0
// =============================================================================

#[test]
fn test_field_access_comparison() {
    let source = r#"
pub struct Data { pub value: f32 }

impl Data {
    pub fn is_small(self) -> bool {
        self.value < 1.0
    }
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32"),
        "self.value < 1.0 should generate 1.0_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Nested field access: self.velocity.x != 0.0 (game pattern from physics_body.wj)
// =============================================================================

#[test]
fn test_nested_field_access_comparison() {
    let source = r#"
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 { Vec3 { x, y, z } }
}

pub struct PhysicsBody { pub velocity: Vec3 }

impl PhysicsBody {
    pub fn has_x_velocity(self) -> bool {
        self.velocity.x != 0.0
    }
    pub fn has_z_velocity(self) -> bool {
        self.velocity.z != 0.0
    }
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.0_f32"),
        "self.velocity.x != 0.0 should generate 0.0_f32 (nested FieldAccess), got:\n{}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Nested field access comparison should NOT use f64, got:\n{}",
        output
    );
}

// =============================================================================
// Nested field: self.settings.gamma != 1.0 (post_processing pattern)
// =============================================================================

#[test]
fn test_nested_settings_gamma_comparison() {
    let source = r#"
pub struct ColorGradingSettings { pub gamma: f32 }

pub struct ColorGrading { pub settings: ColorGradingSettings }

impl ColorGrading {
    pub fn process(self) -> bool {
        if self.settings.gamma != 1.0 {
            return true
        }
        return false
    }
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("1.0_f32"),
        "self.settings.gamma != 1.0 should generate 1.0_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "Settings gamma comparison should NOT use f64, got:\n{}",
        output
    );
}

// =============================================================================
// Triple nested: self.camera.position.x/y/z != 0.0 (quick_start pattern)
// =============================================================================

#[test]
fn test_triple_nested_camera_position_comparison() {
    let source = r#"
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 { Vec3 { x, y, z } }
}

pub struct Camera { pub position: Vec3 }

pub struct Game { pub camera: Camera }

impl Game {
    pub fn is_ready(self) -> bool {
        self.camera.position.x != 0.0 || self.camera.position.y != 0.0 || self.camera.position.z != 0.0
    }
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.0_f32"),
        "self.camera.position.x != 0.0 should generate 0.0_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("0.0_f64"),
        "Camera position comparison should NOT use f64, got:\n{}",
        output
    );
}
