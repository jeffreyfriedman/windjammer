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

//! TDD: All generated Rust structs MUST have #[repr(C)]
//!
//! Bug: Generated Rust structs lacked #[repr(C)], allowing the Rust compiler
//! to reorder fields for optimization. This corrupted GPU uniform buffer
//! layouts where byte-order matters (to_bytes() serializes in declaration order,
//! but without repr(C) the actual memory layout could differ).
//!
//! Root cause of persistent orange rendering in Breach Protocol: material colors
//! and lighting parameters were scrambled by field reordering.
//!
//! Fix: The Windjammer compiler now emits #[repr(C)] on ALL struct definitions,
//! guaranteeing field order matches declaration order in memory.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_simple_struct_has_repr_c() {
    let source = r#"
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
"#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("#[repr(C)]"),
        "Generated Rust struct should have #[repr(C)] for guaranteed field order.\nGenerated:\n{}",
        output
    );
    let repr_c_pos = output.find("#[repr(C)]").unwrap();
    let struct_pos = output.find("pub struct Point").unwrap();
    assert!(
        repr_c_pos < struct_pos,
        "#[repr(C)] should appear BEFORE the struct definition"
    );
}

#[test]
fn test_gpu_uniform_struct_has_repr_c() {
    let source = r#"
pub struct LightingUniforms {
    pub sun_dir_x: f32,
    pub sun_dir_y: f32,
    pub sun_dir_z: f32,
    pub _pad0: f32,
    pub sun_color_r: f32,
    pub sun_color_g: f32,
    pub sun_color_b: f32,
    pub sun_intensity: f32,
}
"#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("#[repr(C)]"),
        "GPU uniform struct MUST have #[repr(C)] to prevent field reordering.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_private_struct_has_repr_c() {
    let source = r#"
struct InternalData {
    pub value: i32,
    pub count: i32,
}
"#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("#[repr(C)]"),
        "Even private structs need #[repr(C)] for consistent layout.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_struct_with_derive_has_repr_c() {
    let source = r#"
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
"#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("#[repr(C)]"),
        "Struct with auto-derive should also have #[repr(C)].\nGenerated:\n{}",
        output
    );
    assert!(
        output.contains("#[derive("),
        "Struct should still have auto-derive traits.\nGenerated:\n{}",
        output
    );
}
