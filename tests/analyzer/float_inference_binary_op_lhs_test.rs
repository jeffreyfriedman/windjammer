/// TDD: Float literal inference for binary operations with typed LHS
///
/// BUG: `x * 1.0` where `x: f32` generates `1.0_f64`, causing E0277 trait bound errors.
/// FIX: When LHS type is known, infer RHS float literal from LHS (LHS → RHS propagation).
///
/// Complements existing RHS → LHS inference. Adds bidirectional constraint flow.
#[path = "../common/test_utils.rs"]
mod test_utils;

// =============================================================================
// Basic operators: LHS (f32) → RHS literal
// =============================================================================

#[test]
fn test_multiplication_x_times_1_0() {
    let source = r#"
pub fn scale(x: f32) -> f32 {
    x * 1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32"),
        "x * 1.0 where x: f32 should generate 1.0_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "Should not generate 1.0_f64 when LHS is f32, got:\n{}",
        output
    );
}

#[test]
fn test_addition_x_plus_1_0() {
    let source = r#"
pub fn add_one(x: f32) -> f32 {
    x + 1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32"),
        "x + 1.0 where x: f32 should generate 1.0_f32, got:\n{}",
        output
    );
}

#[test]
fn test_division_x_over_2_0() {
    let source = r#"
pub fn half(x: f32) -> f32 {
    x / 2.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("2.0_f32"),
        "x / 2.0 where x: f32 should generate 2.0_f32, got:\n{}",
        output
    );
}

#[test]
fn test_subtraction_x_minus_0_5() {
    let source = r#"
pub fn sub(x: f32) -> f32 {
    x - 0.5
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("0.5_f32"),
        "x - 0.5 where x: f32 should generate 0.5_f32, got:\n{}",
        output
    );
}

#[test]
fn test_comparison_x_gt_1_0() {
    let source = r#"
pub fn is_greater(x: f32) -> bool {
    x > 1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32"),
        "x > 1.0 where x: f32 should generate 1.0_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Compound assignment: x *= 1.5
// =============================================================================

#[test]
fn test_compound_assignment_x_times_eq_1_5() {
    let source = r#"
pub fn scale_in_place(mut x: f32) -> f32 {
    x *= 1.5
    x
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.5_f32"),
        "x *= 1.5 where x: f32 should generate 1.5_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Chained operations: x * 1.0 + 2.0 → both literals f32
// =============================================================================

#[test]
fn test_chained_binary_ops() {
    let source = r#"
pub fn chained(x: f32) -> f32 {
    x * 1.0 + 2.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") && output.contains("2.0_f32"),
        "x * 1.0 + 2.0 should generate both literals as f32, got:\n{}",
        output
    );
}

// =============================================================================
// Method call result: obj.get_value() * 1.0
// =============================================================================

#[test]
fn test_method_call_result_times_literal() {
    let source = r#"
struct Holder { value: f32 }

impl Holder {
    pub fn get_value(self) -> f32 {
        self.value
    }
}

pub fn scale_holder(h: Holder) -> f32 {
    h.get_value() * 1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32"),
        "get_value() * 1.0 where get_value returns f32 should generate 1.0_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Field access: self.x * 0.5
// =============================================================================

#[test]
fn test_field_access_times_literal() {
    let source = r#"
pub struct Point { pub x: f32, pub y: f32 }

impl Point {
    pub fn scaled(self) -> Point {
        Point {
            x: self.x * 0.5,
            y: self.y * 2.0,
        }
    }
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("0.5_f32") && output.contains("2.0_f32"),
        "self.x * 0.5 and self.y * 2.0 should generate f32 literals, got:\n{}",
        output
    );
}

// =============================================================================
// Variable (inferred f32): let x: f32 = 1.0; x + 2.0
// =============================================================================

#[test]
fn test_variable_times_literal() {
    let source = r#"
pub fn compute() -> f32 {
    let x: f32 = 10.0
    x * 0.5
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("0.5_f32"),
        "x * 0.5 where x: f32 should generate 0.5_f32, got:\n{}",
        output
    );
}

// =============================================================================
// Unconstrained LHS: 1.0 + 2.0 (no typed context) → default f64
// =============================================================================

#[test]
fn test_unconstrained_defaults_to_f64() {
    let source = r#"
pub fn unconstrained() -> f64 {
    let x = 1.0 + 2.0
    x
}
"#;
    let output = test_utils::compile_single(source);
    // When return type is f64, literals should be f64
    assert!(
        output.contains("_f64"),
        "Unconstrained 1.0 + 2.0 with f64 return should use f64, got:\n{}",
        output
    );
}

// =============================================================================
// Intermediate variable: dist_sq * 1.414 (dist_sq inferred from dx+dy)
// =============================================================================

#[test]
fn test_intermediate_variable_times_literal() {
    let source = r#"
pub fn get_distance(x: f32, y: f32) -> f32 {
    let dx = x * x
    let dy = y * y
    let dist_sq = dx + dy
    dist_sq * 1.414
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.414_f32"),
        "dist_sq * 1.414 where dist_sq is f32 should generate 1.414_f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.414_f64"),
        "Should NOT generate 1.414_f64, got:\n{}",
        output
    );
}
