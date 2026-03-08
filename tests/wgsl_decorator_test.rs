// TDD Test: WGSL decorators should not appear in Rust output
// Bug: @vertex, @fragment, @compute are transpiled to #[vertex], etc.
// which are invalid Rust attributes

use windjammer::*;

#[test]
fn test_wgsl_vertex_decorator_stripped_in_rust() {
    let source = r#"
@vertex
pub fn vs_main(position: vec3<float>) -> vec4<float> {
    vec4(position.x, position.y, position.z, 1.0)
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed);

    // DEBUG: Print generated code
    eprintln!("Generated Rust code:\n{}", rust_code);

    // WGSL decorators should NOT appear in Rust output
    assert!(
        !rust_code.contains("#[vertex]"),
        "WGSL @vertex decorator should not appear in Rust code"
    );
    
    // But the function itself should still be generated
    assert!(
        rust_code.contains("pub fn vs_main"),
        "Function should still be generated"
    );
}

#[test]
fn test_wgsl_fragment_decorator_stripped_in_rust() {
    let source = r#"
@fragment
pub fn fs_main(color: vec4<float>) -> vec4<float> {
    color
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed);

    assert!(
        !rust_code.contains("#[fragment]"),
        "WGSL @fragment decorator should not appear in Rust code"
    );
    assert!(
        rust_code.contains("pub fn fs_main"),
        "Function should still be generated"
    );
}

#[test]
fn test_wgsl_compute_decorator_stripped_in_rust() {
    let source = r#"
@compute
pub fn cs_main(workgroup_id: vec3<uint>) {
    // Compute shader logic
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed);

    assert!(
        !rust_code.contains("#[compute]"),
        "WGSL @compute decorator should not appear in Rust code"
    );
    assert!(
        rust_code.contains("pub fn cs_main"),
        "Function should still be generated"
    );
}

#[test]
fn test_normal_decorators_still_work() {
    let source = r#"
@test
pub fn test_something() {
    assert(true)
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed);

    // Normal decorators like @test should still generate #[test]
    assert!(
        rust_code.contains("#[test]"),
        "@test decorator should still generate #[test] attribute"
    );
}
