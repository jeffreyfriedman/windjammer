/// TDD: Comprehensive float inference for all literal contexts
///
/// Covers: function args, compound assign, return, method calls, nested expressions,
/// Index (arr[i]), chained FieldAccess (self.player.position.x), const/static.

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

/// Function arguments: scale(10.0, 2.0) where params are f32
#[test]
fn test_function_argument_inference() {
    let source = r#"
fn scale(value: f32, factor: f32) -> f32 {
    value * factor
}

fn test() {
    let result = scale(10.0, 2.0)
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(
        (rust.contains("10.0_f32") || rust.contains("10_f32"))
            && (rust.contains("2.0_f32") || rust.contains("2_f32")),
        "scale(10.0, 2.0) should generate _f32 for both args, got:\n{}",
        rust
    );
    assert!(
        !rust.contains("10.0_f64") && !rust.contains("2.0_f64"),
        "Should not generate f64 when params are f32, got:\n{}",
        rust
    );
}

/// Compound assignment: x += 1.0 where x is f32
#[test]
fn test_compound_assignment_inference() {
    let source = r#"
fn test() {
    let mut x: f32 = 0.0
    x += 1.0
    x *= 2.0
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("1.0_f32") || rust.contains("1_f32"),
        "x += 1.0 should infer 1.0 as f32, got:\n{}",
        rust
    );
    assert!(
        rust.contains("2.0_f32") || rust.contains("2_f32"),
        "x *= 2.0 should infer 2.0 as f32, got:\n{}",
        rust
    );
}

/// Return type: return 10.5 where return type is f32
#[test]
fn test_return_type_inference() {
    let source = r#"
fn get_speed() -> f32 {
    return 10.5
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("10.5_f32"),
        "return 10.5 should infer as f32 from return type, got:\n{}",
        rust
    );
}

/// Method call: self.update(2.0) where param is f32
#[test]
fn test_method_call_arg_inference() {
    let source = r#"
struct Widget { value: f32 }

impl Widget {
    fn set_value(self, x: f32) { }
}

fn main() {
    let w = Widget { value: 0.0 }
    w.set_value(2.0)
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("2.0_f32") || rust.contains("2_f32"),
        "w.set_value(2.0) should generate _f32 when param is f32, got:\n{}",
        rust
    );
}

/// Nested expressions: (x + 1.0) * 2.0 where x is f32
#[test]
fn test_nested_expressions_inference() {
    let source = r#"
fn compute(x: f32) -> f32 {
    (x + 1.0) * 2.0
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(
        (rust.contains("1.0_f32") || rust.contains("1_f32"))
            && (rust.contains("2.0_f32") || rust.contains("2_f32")),
        "Nested (x + 1.0) * 2.0 should infer both literals as f32, got:\n{}",
        rust
    );
}

/// Index: arr[i] / 2.0 when arr: Vec<f32>
#[test]
fn test_index_div_literal_inference() {
    let source = r#"
pub fn half_element(arr: Vec<f32>, i: i32) -> f32 {
    arr[i] / 2.0
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("2.0_f32") || rust.contains("2_f32"),
        "arr[i] / 2.0 should infer 2.0 as f32, got:\n{}",
        rust
    );
}

/// Chained FieldAccess: self.player.position.x + offset_x when offset_x = 0.0
#[test]
fn test_chained_field_access_var_inference() {
    let source = r#"
pub struct Vec3 { x: f32, y: f32, z: f32 }
pub struct Player { position: Vec3 }
pub struct Game { player: Player }

impl Game {
    fn camera_x(self) -> f32 {
        let offset_x = 0.0
        self.player.position.x + offset_x
    }
}
"#;
    let rust = compile_and_get_rust(source);
    // offset_x = 0.0 must be f32 so it matches position.x in the addition
    assert!(
        rust.contains("0.0_f32") || rust.contains("0_f32"),
        "0.0 in offset_x = 0.0 should infer f32 from usage in self.player.position.x + offset_x, got:\n{}",
        rust
    );
}

/// Const: const MAX: f32 = 999.0
#[test]
fn test_const_float_inference() {
    let source = r#"
const MAX_SPEED: f32 = 999.0

fn get_max() -> f32 {
    MAX_SPEED
}
"#;
    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("999.0_f32") || rust.contains("999_f32"),
        "const MAX_SPEED: f32 = 999.0 should generate 999.0_f32, got:\n{}",
        rust
    );
}
