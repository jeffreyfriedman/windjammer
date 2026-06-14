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

//! Trait methods with non-Self returns use `&self` in signatures (object-safe); impls must match.

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
    let (analyzed_functions, analyzed_structs, analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.set_source_file(filename);
    generator.set_analyzed_trait_methods(analyzed_trait_methods);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn trait_impl_render_self_generates_owned_receiver() {
    let src = r#"
pub trait Renderable {
    fn render(self) -> string
}

pub struct Panel {
    pub title: string,
}

impl Renderable for Panel {
    fn render(self) -> string {
        self.title
    }
}
"#;
    let out = parse_and_generate_rust(src, "trait_render.wj");
    assert!(
        out.contains("fn render(&self) -> String"),
        "trait and impl must use &self for non-Self return (object-safe):\n{}",
        out
    );
}
