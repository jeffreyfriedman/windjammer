/// TDD test for constructor functions (associated functions)
/// BUG: Constructor functions incorrectly getting self parameter added
/// 
/// Example:
/// ```windjammer
/// impl Tilemap {
///     pub fn new(width: i32, height: i32) -> Tilemap {
///         Tilemap { width, height, tiles: Vec::new() }
///     }
/// }
/// ```
/// 
/// EXPECTED: pub fn new(width: i32, height: i32) -> Tilemap
/// ACTUAL: pub fn new(&mut self, width: i32, height: i32) -> Tilemap ❌

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
fn test_constructor_no_self_param() {
    let code = r#"
struct Tilemap {
    width: i32,
    height: i32,
}

impl Tilemap {
    pub fn new(width: i32, height: i32) -> Tilemap {
        Tilemap { width, height }
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    // DEBUG: Print generated code
    eprintln!("Generated:\n{}", generated);
    
    // ASSERT: Constructor should NOT have self parameter
    assert!(!generated.contains("fn new(&self"), 
        "Constructor should NOT have &self parameter!\nGenerated:\n{}", generated);
    assert!(!generated.contains("fn new(&mut self"), 
        "Constructor should NOT have &mut self parameter!\nGenerated:\n{}", generated);
    
    // ASSERT: Constructor should match source signature
    assert!(generated.contains("pub fn new(width: i32, height: i32) -> Tilemap"), 
        "Constructor should match source signature!\nGenerated:\n{}", generated);
}

#[test]
fn test_method_can_have_self_param() {
    let code = r#"
struct Tilemap {
    width: i32,
    height: i32,
}

impl Tilemap {
    pub fn get_width(self) -> i32 {
        self.width
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    // ASSERT: Regular method should have self parameter
    assert!(generated.contains("get_width(&self)"), 
        "Regular method should have &self parameter!\nGenerated:\n{}", generated);
}

#[test]
fn test_constructor_with_multiple_params() {
    let code = r#"
struct Config {
    name: string,
    version: i32,
    debug: bool,
}

impl Config {
    pub fn new(name: string, version: i32, debug: bool) -> Config {
        Config { name, version, debug }
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    // ASSERT: Constructor with many params should NOT have self
    assert!(!generated.contains("fn new(&"), 
        "Constructor should NOT have self parameter!\nGenerated:\n{}", generated);
}

#[test]
fn test_constructor_with_mutating_methods_in_impl() {
    let code = r#"
struct Tilemap {
    tiles: Vec<Vec<i32> >,
    width: i32,
}

impl Tilemap {
    pub fn new(width: i32) -> Tilemap {
        let mut tiles = Vec::new()
        for row in 0..width {
            tiles.push(Vec::new())
        }
        Tilemap { tiles, width }
    }
    
    pub fn set_tile(self, row: i32, col: i32, value: i32) {
        self.tiles[row as int][col as int] = value
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    eprintln!("Generated:\n{}", generated);
    
    // ASSERT: Constructor should NOT have self even when other methods mutate
    assert!(!generated.contains("fn new(&self"), 
        "Constructor should NOT have &self even when other methods mutate!\nGenerated:\n{}", generated);
    assert!(!generated.contains("fn new(&mut self"), 
        "Constructor should NOT have &mut self even when other methods mutate!\nGenerated:\n{}", generated);
    
    // ASSERT: Mutating method should have &mut self
    assert!(generated.contains("set_tile(&mut self"), 
        "Mutating method should have &mut self!\nGenerated:\n{}", generated);
}

