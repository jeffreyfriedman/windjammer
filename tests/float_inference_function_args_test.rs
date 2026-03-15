/// TDD: Float literal inference for function arguments
///
/// Problem: `calculate(1.0, 2.0)` generates `1.0_f64` when parameters are `f32`, causing E0308.
/// Goal: Infer float literal type from function parameter signature.
///
/// Constraint: Only infer when parameter type is KNOWN at analysis time.

use windjammer::*;

fn compile_and_assert(source: &str, assertions: impl Fn(&str)) {
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
    let rust_code = generator.generate_program(&program, &analyzed);

    assertions(&rust_code);
}

#[test]
fn test_simple_function_call_single_arg() {
    // foo(1.0) where fn foo(x: f32)
    let source = r#"
fn foo(x: f32) { }
fn main() {
    foo(1.0)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("1.0_f32") || rust_code.contains("1_f32"),
            "foo(1.0) should generate _f32 when param is f32, got:\n{}",
            rust_code
        );
        assert!(
            !rust_code.contains("1.0_f64"),
            "Should not generate f64 when param is f32, got:\n{}",
            rust_code
        );
    });
}

#[test]
fn test_simple_function_call_multiple_args() {
    // bar(1.0, 2.0) where fn bar(x: f32, y: f32)
    let source = r#"
fn bar(x: f32, y: f32) { }
fn main() {
    bar(1.0, 2.0)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            (rust_code.contains("1.0_f32") || rust_code.contains("1_f32"))
                && (rust_code.contains("2.0_f32") || rust_code.contains("2_f32")),
            "bar(1.0, 2.0) should generate _f32 for both args, got:\n{}",
            rust_code
        );
        assert!(
            !rust_code.contains("_f64"),
            "Should not generate f64 when params are f32, got:\n{}",
            rust_code
        );
    });
}

#[test]
fn test_method_call_with_float_arg() {
    // obj.method(1.0) where fn method(&self, x: f32)
    let source = r#"
struct Widget { value: f32 }

impl Widget {
    fn set_value(self, x: f32) { }
}

fn main() {
    let w = Widget { value: 0.0 }
    w.set_value(1.0)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("1.0_f32") || rust_code.contains("1_f32"),
            "w.set_value(1.0) should generate _f32 when param is f32, got:\n{}",
            rust_code
        );
    });
}

#[test]
fn test_mixed_types_f32_and_int() {
    // baz(1.0, 2) where fn baz(x: f32, y: i32)
    let source = r#"
fn baz(x: f32, y: i32) { }
fn main() {
    baz(1.0, 2)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("1.0_f32") || rust_code.contains("1_f32"),
            "baz(1.0, 2) should generate 1.0_f32 for first arg, got:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains(", 2)") || rust_code.contains(", 2 "),
            "Second arg should remain as int 2, got:\n{}",
            rust_code
        );
    });
}

#[test]
fn test_associated_function_call() {
    // Vec3::new(1.0, 2.0, 3.0) where fn new(x: f32, y: f32, z: f32)
    let source = r#"
struct Vec3 { x: f32, y: f32, z: f32 }

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

fn main() {
    let v = Vec3::new(1.0, 2.0, 3.0)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("1.0_f32") && rust_code.contains("2.0_f32") && rust_code.contains("3.0_f32"),
            "Vec3::new(1.0, 2.0, 3.0) should generate _f32 for all args, got:\n{}",
            rust_code
        );
    });
}

#[test]
fn test_f64_params_infer_f64() {
    // When params are f64, literals should be f64
    let source = r#"
fn compute(x: f64, y: f64) -> f64 {
    x + y
}
fn main() {
    compute(1.0, 2.0)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("1.0_f64") && rust_code.contains("2.0_f64"),
            "compute(1.0, 2.0) with f64 params should generate _f64, got:\n{}",
            rust_code
        );
    });
}

#[test]
fn test_nested_call_with_float_arg() {
    // outer(inner(1.0)) where inner returns f32, outer takes f32
    let source = r#"
fn inner(x: f32) -> f32 { x }
fn outer(x: f32) { }
fn main() {
    outer(inner(1.0))
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("1.0_f32") || rust_code.contains("1_f32"),
            "inner(1.0) should generate _f32, got:\n{}",
            rust_code
        );
    });
}

#[test]
fn test_unconstrained_literal_has_type_suffix() {
    // Unconstrained literal gets a type suffix (f32 or f64 depending on context/default)
    let source = r#"
fn main() {
    let x = 1.0
}
"#;
    compile_and_assert(source, |rust_code| {
        // Unconstrained 1.0 should have type suffix to avoid E0689
        assert!(
            rust_code.contains("_f64") || rust_code.contains("_f32"),
            "Unconstrained 1.0 should have type suffix, got:\n{}",
            rust_code
        );
    });
}

#[test]
fn test_calculate_two_f32_args() {
    // Direct repro of user's problem: calculate(1.0, 2.0) with f32 params
    let source = r#"
fn calculate(x: f32, y: f32) -> f32 {
    x + y
}
fn main() {
    let result = calculate(1.0, 2.0)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            (rust_code.contains("1.0_f32") || rust_code.contains("1_f32"))
                && (rust_code.contains("2.0_f32") || rust_code.contains("2_f32")),
            "calculate(1.0, 2.0) should generate _f32 for both args (E0308 fix), got:\n{}",
            rust_code
        );
        assert!(
            !rust_code.contains("1.0_f64") && !rust_code.contains("2.0_f64"),
            "Should NOT generate f64 when params are f32, got:\n{}",
            rust_code
        );
    });
}
