/// TDD test for string literal conversion in struct constructors
/// BUG: Compiler adds .to_string() even when function expects &str
/// 
/// Example:
/// ```windjammer
/// struct Sound {}
/// impl Sound {
///     pub fn load(path: &str) -> Sound { ... }
/// }
/// struct SoundLibrary {
///     jump: Sound,
/// }
/// impl SoundLibrary {
///     pub fn new() -> SoundLibrary {
///         SoundLibrary { jump: Sound::load("assets/jump.ogg") }
///     }
/// }
/// ```
/// 
/// EXPECTED: Sound::load("assets/jump.ogg")
/// ACTUAL: Sound::load("assets/jump.ogg".to_string()) ❌ Type mismatch!

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
fn test_string_literal_in_struct_constructor_with_str_param() {
    let code = r#"
struct Sound {
    handle: int,
}

impl Sound {
    pub fn load(path: &str) -> Sound {
        Sound { handle: 0 }
    }
}

struct SoundLibrary {
    jump: Sound,
}

impl SoundLibrary {
    pub fn new() -> SoundLibrary {
        SoundLibrary { jump: Sound::load("assets/jump.ogg") }
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    // ASSERT: Should NOT add .to_string() for &str parameters
    assert!(!generated.contains("\"assets/jump.ogg\".to_string()"), 
        "Compiler should NOT add .to_string() when function expects &str!");
    
    // ASSERT: Should pass string literal directly as &str
    assert!(generated.contains("Sound::load(\"assets/jump.ogg\")"),
        "Compiler should pass string literals directly as &str!");
}

#[test]
fn test_string_literal_in_nested_struct_constructor() {
    let code = r#"
struct Sound {
    path: string,
}

impl Sound {
    pub fn new(path: &str) -> Sound {
        Sound { path: path.to_string() }
    }
}

struct Library {
    sounds: Vec<Sound>,
}

impl Library {
    pub fn create() -> Library {
        Library {
            sounds: vec![
                Sound::new("jump.ogg"),
                Sound::new("land.ogg"),
                Sound::new("coin.ogg"),
            ]
        }
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    // ASSERT: Should NOT add .to_string() for &str parameters in vec! macro
    assert!(!generated.contains("\"jump.ogg\".to_string()"), 
        "Compiler should NOT add .to_string() when function expects &str!");
    assert!(!generated.contains("\"land.ogg\".to_string()"), 
        "Compiler should NOT add .to_string() when function expects &str!");
    assert!(!generated.contains("\"coin.ogg\".to_string()"), 
        "Compiler should NOT add .to_string() when function expects &str!");
}

#[test]
fn test_string_owned_parameter_does_add_to_string() {
    let code = r#"
struct Config {
    name: string,
}

impl Config {
    pub fn new(name: string) -> Config {
        Config { name }
    }
}

struct App {
    config: Config,
}

impl App {
    pub fn create() -> App {
        App { config: Config::new("MyApp") }
    }
}
"#;
    
    let program = parse_code(code);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let generated = generator.generate_program(&program, &analyzed_functions);
    
    // ASSERT: SHOULD add .to_string() for owned String parameters
    assert!(generated.contains("\"MyApp\".to_string()"), 
        "Compiler SHOULD add .to_string() when function expects owned String!");
}

