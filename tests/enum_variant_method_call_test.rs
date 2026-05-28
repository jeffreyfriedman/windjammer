#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! Enum variant instance methods and variables named `io` must not codegen as `::` module calls.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate_rust(code: &str, filename: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.set_source_file(filename);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn enum_variant_method_call_uses_dot_not_double_colon() {
    let src = r#"
pub enum ShaderFile { HiZCull, HiZDownsample }

impl ShaderFile {
    pub fn to_path(self) -> string {
        match self {
            ShaderFile::HiZCull => "hiz.wjsl",
            ShaderFile::HiZDownsample => "hiz_downsample.wjsl",
        }
    }
}

pub fn path() -> string {
    ShaderFile::HiZCull.to_path()
}
"#;
    let out = parse_and_generate_rust(src, "enum_variant_method.wj");
    assert!(
        !out.contains("ShaderFile::HiZCull::to_path()"),
        "variant must not be treated as module:\n{}",
        out
    );
    assert!(
        !out.contains("ShaderFile::HiZDownsample::to_path()"),
        "qualified variant identifier must use dot:\n{}",
        out
    );
    assert!(
        out.contains(".to_path()"),
        "expected instance method call:\n{}",
        out
    );
}

#[test]
fn variable_named_io_uses_dot_method_not_std_io() {
    let src = r#"
pub struct InputState { has_fire: bool }

impl InputState {
    pub fn new() -> InputState { InputState { has_fire: false } }
    pub fn has_action(self, _a: i32) -> bool { self.has_fire }
}

pub fn test_io() -> bool {
    let mut io = InputState::new()
    io.has_action(1)
}
"#;
    let out = parse_and_generate_rust(src, "io_variable.wj");
    assert!(
        !out.contains("io::has_action"),
        "variable io must not codegen as module:\n{}",
        out
    );
    assert!(out.contains("io.has_action"), "expected:\n{}", out);
}
