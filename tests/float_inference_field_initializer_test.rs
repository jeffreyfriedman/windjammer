//! TDD: Float literal inference for struct field initializers
//!
//! Bug: `MyStruct { cost: 1.0 }` generates `1.0_f64` when field type is f32, causing E0308.
//! Goal: Infer float type from struct field definition when literal appears in field initializer.

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

/// Basic case: Struct with f32 field, bare 1.0 literal → 1.0_f32
#[test]
fn test_f32_field_bare_literal() {
    let source = r#"
pub struct MyStruct {
    pub cost: f32,
}

pub fn create() -> MyStruct {
    MyStruct { cost: 1.0 }
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("cost: 1.0_f32") || rust.contains("cost: 1.0f32"),
        "Expected 1.0_f32 when field is f32. Got:\n{}",
        rust
    );
    assert!(
        !rust.contains("cost: 1.0_f64"),
        "Should NOT generate f64 when field is f32. Got:\n{}",
        rust
    );
}

/// Nested struct: Vec3 { x: 1.0, y: 2.0, z: 3.0 } → all f32
#[test]
fn test_nested_struct_vec3_all_f32() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn origin() -> Vec3 {
    Vec3 { x: 1.0, y: 2.0, z: 3.0 }
}
"#;

    let rust = compile_and_get_rust(source);
    for (lit, field) in [("1.0", "x"), ("2.0", "y"), ("3.0", "z")] {
        assert!(
            rust.contains(&format!("{}: {}_f32", field, lit)) || rust.contains(&format!("{}: {}f32", field, lit)),
            "Expected {}_f32 for field {}. Got:\n{}",
            lit,
            field,
            rust
        );
    }
    assert!(
        !rust.contains("_f64"),
        "Should not use f64 for Vec3 fields. Got:\n{}",
        rust
    );
}

/// Struct in Vec::push: cells.push(Cell { cost: 1.0 })
#[test]
fn test_struct_in_vec_push() {
    let source = r#"
pub struct AStarCell {
    pub walkable: bool,
    pub cost: f32,
}

pub fn create_cells() -> Vec<AStarCell> {
    let mut cells = Vec::new()
    cells.push(AStarCell { walkable: true, cost: 1.0 })
    cells
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("cost: 1.0_f32") || rust.contains("cost: 1.0f32"),
        "Expected 1.0_f32 in Vec::push. Got:\n{}",
        rust
    );
    assert!(
        !rust.contains("cost: 1.0_f64"),
        "Should NOT generate f64. Got:\n{}",
        rust
    );
}

/// f64 field: literal should be f64 (ensure we don't over-constrain)
#[test]
fn test_f64_field_gets_f64() {
    let source = r#"
pub struct Precise {
    pub value: f64,
}

pub fn create() -> Precise {
    Precise { value: 3.14159 }
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("value: 3.14159_f64") || rust.contains("value: 3.14159f64"),
        "Expected 3.14159_f64 when field is f64. Got:\n{}",
        rust
    );
}

/// Mixed f32/f64 in same struct
#[test]
fn test_mixed_f32_f64_fields() {
    let source = r#"
pub struct Mixed {
    pub low: f32,
    pub high: f64,
}

pub fn create() -> Mixed {
    Mixed { low: 0.5, high: 1.5 }
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("low: 0.5_f32") || rust.contains("low: 0.5f32"),
        "low (f32) should get _f32. Got:\n{}",
        rust
    );
    assert!(
        rust.contains("high: 1.5_f64") || rust.contains("high: 1.5f64"),
        "high (f64) should get _f64. Got:\n{}",
        rust
    );
}

/// TDD E0308: Struct with tuple fields - Keyframe { rotation: (0.0, 0.0, 0.0, 1.0) }
/// Bug: Tuple elements in struct literal default to f64 when field type is (f32, f32, f32, f32)
#[test]
fn test_struct_tuple_field_f32() {
    let source = r#"
pub struct Keyframe {
    pub rotation: (f32, f32, f32, f32),
    pub scale: (f32, f32, f32),
}

pub fn default_keyframe() -> Keyframe {
    Keyframe {
        rotation: (0.0, 0.0, 0.0, 1.0),
        scale: (1.0, 1.0, 1.0),
    }
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        !rust.contains("_f64"),
        "Tuple fields (rotation, scale) should infer f32 from struct. Got:\n{}",
        rust
    );
    assert!(
        rust.contains("0.0_f32") || rust.contains("0.0f32"),
        "Expected f32 literals in tuple. Got:\n{}",
        rust
    );
}

/// TDD E0308: Type alias Quat = (f32, f32, f32, f32) in struct field
/// Bug: rotation: Quat with type alias generates f64 when alias not resolved
#[test]
fn test_struct_tuple_field_with_type_alias() {
    let source = r#"
pub type Quat = (f32, f32, f32, f32)

pub struct Keyframe {
    pub rotation: Quat,
    pub scale: (f32, f32, f32),
}

pub fn default_keyframe() -> Keyframe {
    Keyframe {
        rotation: (0.0, 0.0, 0.0, 1.0),
        scale: (1.0, 1.0, 1.0),
    }
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        !rust.contains("_f64"),
        "Tuple fields with type alias Quat should infer f32. Got:\n{}",
        rust
    );
    assert!(
        rust.contains("0.0_f32") || rust.contains("0.0f32"),
        "Expected f32 literals in rotation tuple. Got:\n{}",
        rust
    );
}

/// Unconstrained literal (no struct context) defaults to f64
#[test]
fn test_unconstrained_defaults_to_f64() {
    let source = r#"
pub fn standalone() -> f64 {
    2.718
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("2.718_f64") || rust.contains("2.718f64"),
        "Unconstrained literal should default to f64. Got:\n{}",
        rust
    );
}
