/// TDD: Remaining float arithmetic patterns (E0277 fix)
///
/// **Patterns from game build errors:**
/// 1. Cast / literal: `current_g as f32 / 10.0` → 10.0 should be f32
/// 2. Struct field / literal: `self.size.x / 2.0` → 2.0 should be f32 (nested FieldAccess)
/// 3. Nested division: `(member_index as f32) * (6.28318 / self.members.len() as f32)` → 6.28318 f32
/// 4. Method result * literal: `(seed * 1234.567).sin() * 3.14159265 * 2.0` → all f32

use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    if !float_inference.errors.is_empty() {
        panic!("Float inference errors: {:?}", float_inference.errors);
    }

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

/// Pattern 1: Cast / literal - pathfinder.wj: current_g as f32 / 10.0
#[test]
fn test_cast_div_literal_infers_f32() {
    let source = r#"
pub fn path_cost(current_g: i32) -> f32 {
    current_g as f32 / 10.0
}
"#;

    let output = compile_and_get_rust(source);
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

    let output = compile_and_get_rust(source);
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

    let output = compile_and_get_rust(source);
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

    let output = compile_and_get_rust(source);
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

    let output = compile_and_get_rust(source);
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

    let output = compile_and_get_rust(source);
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

    let output = compile_and_get_rust(source);
    assert!(
        output.contains("2.0_f32") || output.contains("2_f32"),
        "2.0 in width / 2.0 should be f32 when width is f32 param:\n{}",
        output
    );
}
