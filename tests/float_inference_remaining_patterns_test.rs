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

// Pattern 1: Cast / literal - pathfinder.wj: current_g as f32 / 10.0

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_cast_div_literal_infers_f32() {
    let source = r#"
pub fn path_cost(current_g: i32) -> f32 {
    current_g as f32 / 10.0
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("10.0_f32") || output.contains("10_f32"),
        "10.0 in 'cast / literal' should be f32:\n{}",
        output
    );
    assert!(
        !output.contains("10.0_f64"),
        "Should not generate f64 when dividing f32 by literal:\n{}",
        output
    );
}

/// Pattern 2: Struct field / literal - physics_body.wj: self.size.x / 2.0
#[test]
fn test_nested_field_access_div_literal_infers_f32() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct PhysicsBody {
    pub position: Vec3,
    pub size: Vec3,
}

impl PhysicsBody {
    fn bounds(self) -> (i32, i32) {
        let min_x = (self.position.x - self.size.x / 2.0) as i32
        let max_x = (self.position.x + self.size.x / 2.0) as i32
        (min_x, max_x)
    }
}
"#;

    let output = test_utils::compile_single(source);
    // self.size.x / 2.0 - 2.0 must be f32 to match self.size.x (f32)
    assert!(
        output.contains("2.0_f32") || output.contains("2_f32"),
        "2.0 in self.size.x / 2.0 should be f32 (nested FieldAccess):\n{}",
        output
    );
    assert!(
        !output.contains("2.0_f64"),
        "Should not generate f64 for struct field division:\n{}",
        output
    );
}

/// Pattern 3: Nested division - squad_tactics.wj: 6.28318 / self.members.len() as f32
#[test]
fn test_nested_division_literal_infers_f32_from_rhs_cast() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Formation {
    pub members: Vec<i32>,
}

impl Formation {
    fn angle(self, i: i32) -> f32 {
        (i as f32) * (6.28318 / self.members.len() as f32)
    }
}
"#;

    let output = test_utils::compile_single(source);
    // 6.28318 in division with f32 divisor should be f32
    assert!(
        output.contains("6.28318_f32"),
        "6.28318 in 6.28318 / count as f32 should be f32:\n{}",
        output
    );
    assert!(
        !output.contains("6.28318_f64"),
        "Should not generate f64 when RHS of division is f32:\n{}",
        output
    );
}

/// Pattern 4: f64 * f32 - theta = 6.28318 * seg as f32 / segments (literal on LHS)
#[test]
fn test_literal_times_cast_div_infers_from_context() {
    let source = r#"
pub fn circle_theta(seg: i32, segments: i32) -> f32 {
    6.28318530718 * (seg as f32) / (segments as f32)
}
"#;

    let output = test_utils::compile_single(source);
    // Result is f32 (return type). Literal 6.28318... must match operands.
    // seg as f32 is f32, segments as f32 is f32. So 6.28318 should be f32.
    assert!(
        output.contains("6.28318530718_f32") || output.contains("6.28318530718f32"),
        "Literal in f32 context should be f32:\n{}",
        output
    );
}

/// Pattern 5: Comparison - physics_body.wj: self.velocity.x != 0.0
#[test]
fn test_comparison_field_literal_infers_f32() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct PhysicsBody {
    pub velocity: Vec3,
}

impl PhysicsBody {
    fn has_movement(self) -> bool {
        self.velocity.x != 0.0
    }
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("0.0_f32") || output.contains("0_f32"),
        "0.0 in comparison with f32 field should be f32:\n{}",
        output
    );
}

/// Pattern 6: Index / literal - arr[i] / 2.0 (Vec<f32> element)
#[test]
fn test_index_div_literal_infers_f32() {
    let source = r#"
pub fn half_element(arr: Vec<f32>, i: i32) -> f32 {
    arr[i] / 2.0
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("2.0_f32") || output.contains("2_f32"),
        "2.0 in arr[i] / 2.0 should be f32 (Index yields Vec element type):\n{}",
        output
    );
}

/// Pattern 7: width / 2.0 - variable from struct field (collision2d.wj)
#[test]
fn test_var_from_param_div_literal_infers_f32() {
    let source = r#"
pub fn half_width(width: f32) -> f32 {
    width / 2.0
}
"#;

    let output = test_utils::compile_single(source);
    assert!(
        output.contains("2.0_f32") || output.contains("2_f32"),
        "2.0 in width / 2.0 should be f32 when width is f32 param:\n{}",
        output
    );
}
