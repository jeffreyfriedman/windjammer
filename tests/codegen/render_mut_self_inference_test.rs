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

//! Methods that call mutating engine APIs on `self.field` must get `&mut self`.

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
fn render_with_frame_call_gets_mut_self() {
    let src = r#"
pub struct Renderer {}

impl Renderer {
    pub fn render_frame_with_dt(self, _dt: f32) {}
}

pub struct Game {
    pub renderer: Renderer,
    pub initialized: bool,
    pub last_dt: f32,
}

impl Game {
    pub fn render(self) {
        if !self.initialized { return }
        let dt = self.last_dt
        self.renderer.render_frame_with_dt(dt)
    }
}
"#;
    let out = parse_and_generate_rust(src, "render_mut.wj");
    assert!(
        out.contains("fn render(&mut self)"),
        "mutating field method call must infer &mut self:\n{}",
        out
    );
}
