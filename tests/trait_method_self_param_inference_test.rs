/// TDD test for trait method self parameter inference
/// BUG: Trait methods with self parameter infer &self instead of self
/// 
/// Example:
/// ```windjammer
/// impl Neg for Vec2 {
///     type Output = Vec2;
///     fn neg(self) -> Vec2 {
///         Vec2::new(-self.x, -self.y)
///     }
/// }
/// ```
/// 
/// EXPECTED: fn neg(self) -> Vec2
/// ACTUAL: fn neg(&self) -> Vec2 ❌

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::parser::{Parser, Program};
use windjammer::lexer::Lexer;
use windjammer::CompilationTarget;

fn parse_code(code: &str) -> Program {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser.parse().unwrap()
}

#[test]
fn test_trait_neg_impl_uses_owned_self() {
    let code = r#"
struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    // ASSERT: Should use owned self, not &self
    assert!(generated.contains("fn neg(self) -> Vec2"), 
        "Trait method should use owned self!\nGenerated:\n{}", generated);
    
    // ASSERT: Should NOT use &self
    assert!(!generated.contains("fn neg(&self) -> Vec2"), 
        "Trait method should NOT use &self!\nGenerated:\n{}", generated);
}

#[test]
fn test_trait_add_impl_uses_owned_self() {
    let code = r#"
struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    // ASSERT: Both self and other should be owned
    assert!(generated.contains("fn add(self, other: Vec2) -> Vec2"), 
        "Trait method should use owned self and other!\nGenerated:\n{}", generated);
}

#[test]
fn test_non_trait_method_can_infer_ref() {
    let code = r#"
struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    // ASSERT: Non-trait methods can infer &self for read-only access
    assert!(generated.contains("length(&self)"), 
        "Non-trait read-only method should infer &self!\nGenerated:\n{}", generated);
}

