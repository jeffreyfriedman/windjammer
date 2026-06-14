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

//! Same-crate module calls (e.g. vnode_ffi) must not wrap strings with windjammer_runtime::ffi.

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
fn module_qualified_call_does_not_use_runtime_string_to_ffi() {
    let src = r#"
use crate::vnode_ffi

pub fn make_div() -> u64 {
    vnode_ffi::vnode_element("div")
}
"#;
    let out = parse_and_generate_rust(src, "vnode_call.wj");
    assert!(
        !out.contains("windjammer_runtime::ffi::string_to_ffi"),
        "internal module calls must not use runtime FFI wrappers:\n{}",
        out
    );
    assert!(
        out.contains("vnode_ffi::vnode_element"),
        "expected direct module call:\n{}",
        out
    );
}
