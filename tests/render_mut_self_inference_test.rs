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
//! Mutation propagates: if `Renderer::render_frame_with_dt` needs `&mut self`,
//! then `Game::render` calling `self.renderer.render_frame_with_dt(dt)` also
//! needs `&mut self`.

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

/// When a field method genuinely mutates (modifies self.field inside its body),
/// calling it on self.field propagates `&mut self` to the outer method.
#[test]
fn render_with_mutating_field_method_gets_mut_self() {
    let src = r#"
pub struct Renderer {
    pub frame_count: i32
}

impl Renderer {
    pub fn render_frame_with_dt(self, _dt: f32) {
        self.frame_count = self.frame_count + 1
    }
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

/// When a field method is read-only (empty body or only reads), calling it
/// on self.field should NOT force &mut self on the outer method.
/// Uses a non-Copy struct (has String field) to test &self inference.
/// Copy types correctly get `self` by value instead.
#[test]
fn render_with_readonly_field_method_gets_ref_self() {
    let src = r#"
pub struct Renderer {
    pub name: string
}

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
    let out = parse_and_generate_rust(src, "render_readonly.wj");
    assert!(
        out.contains("fn render(&self)"),
        "read-only field method call should infer &self, not &mut self:\n{}",
        out
    );
}
