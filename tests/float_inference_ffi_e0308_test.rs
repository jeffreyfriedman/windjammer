/// TDD: Float inference for ffi::module-qualified calls (E0308 fix)
///
/// Bug: ffi::tilemap_check_collision(1.0, 2.0) generates f64 when params are f32.
/// Root cause: Function signature lookup used "ffi::tilemap_check_collision" but
/// metadata stores "tilemap_check_collision" (from ffi/api.wj.meta).
///
/// Fix: 1) Load ffi submodule metadata (ffi/api.wj.meta)
///      2) Register with parent prefix "ffi::" so lookup succeeds
///      3) Fallback: try bare name when module-qualified lookup fails
///      4) Default f32 for unknown-sig float args

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
fn test_same_module_extern_fn_infers_f32() {
    // extern fn in same module - signature from registry
    let source = r#"
extern fn tilemap_check_collision(tilemap_id: u32, x: f32, y: f32, width: f32, height: f32, tile_size: f32) -> bool

fn main() {
    let _ = tilemap_check_collision(0, 1.0, 2.0, 3.0, 4.0, 5.0)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("1.0_f32") && rust_code.contains("2.0_f32"),
            "extern fn with f32 params should infer f32 for literals, got:\n{}",
            rust_code
        );
        assert!(
            !rust_code.contains("1.0_f64"),
            "Should not generate f64 when params are f32, got:\n{}",
            rust_code
        );
    });
}

#[test]
fn test_unknown_function_defaults_float_to_f32() {
    // Call to unknown function (e.g. cross-module) - default float args to f32
    let source = r#"
fn main() {
    // Simulates ffi::unknown_fn(1.0, 2.0) when signature not in registry
    unknown_fn(1.0, 2.0)
}
"#;
    compile_and_assert(source, |rust_code| {
        assert!(
            rust_code.contains("1.0_f32") && rust_code.contains("2.0_f32"),
            "Unknown function should default float args to f32 (game/graphics convention), got:\n{}",
            rust_code
        );
    });
}
