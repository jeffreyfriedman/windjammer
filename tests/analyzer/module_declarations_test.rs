use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
/// TDD Test: Module declarations
///
/// Windjammer needs to support module declarations for proper project structure:
/// - `mod math;` - External module declaration
/// - `pub mod rendering;` - Public external module
/// - `mod utils { ... }` - Inline module
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn compile_code(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _) = analyzer.analyze_program(&program).unwrap();
    // Use regular generator (not module) so main() gets generated for test
    let mut generator = CodeGenerator::new(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_external_module_declaration() {
    let code = r#"
    mod math;
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should generate: mod math;
    assert!(
        output.contains("mod math;"),
        "Should generate external module declaration"
    );
}

#[test]
fn test_public_external_module() {
    let code = r#"
    pub mod rendering;
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should generate: pub mod rendering;
    assert!(
        output.contains("pub mod rendering;"),
        "Should generate public external module declaration"
    );
}

#[test]
fn test_multiple_module_declarations() {
    let code = r#"
    pub mod math;
    pub mod physics;
    mod internal;
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should generate all three declarations
    assert!(output.contains("pub mod math;"), "Should have math module");
    assert!(
        output.contains("pub mod physics;"),
        "Should have physics module"
    );
    assert!(
        output.contains("mod internal;"),
        "Should have internal module"
    );
}

#[test]
fn test_inline_module() {
    let code = r#"
    mod utils {
        pub fn helper() -> int {
            return 42
        }
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should generate inline module with contents
    assert!(
        output.contains("mod utils {"),
        "Should have inline module declaration"
    );
    assert!(
        output.contains("pub fn helper()"),
        "Should have function inside module"
    );
    assert!(output.contains("42"), "Should have function body");
}

#[test]
fn test_public_inline_module() {
    let code = r#"
    pub mod utils {
        pub struct Point { x: int, y: int }
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should generate public inline module
    assert!(
        output.contains("pub mod utils {"),
        "Should have public inline module"
    );
    assert!(
        output.contains("pub struct Point"),
        "Should have struct inside module"
    );
}

#[test]
fn test_mixed_modules_and_items() {
    let code = r#"
    pub mod math;
    
    pub struct Game { name: string }
    
    mod utils {
        pub fn init() {}
    }
    
    pub fn main() {
        println("Hello")
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should generate module declarations alongside other items
    assert!(
        output.contains("pub mod math;"),
        "Should have module declaration"
    );
    assert!(output.contains("pub struct Game"), "Should have struct");
    assert!(output.contains("mod utils {"), "Should have inline module");
    assert!(
        output.contains("pub fn main()"),
        "Should have main function"
    );
}

#[test]
fn test_nested_inline_modules() {
    let code = r#"
    pub mod game {
        pub mod physics {
            pub struct World {}
        }
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should generate nested inline modules
    assert!(
        output.contains("pub mod game {"),
        "Should have outer module"
    );
    assert!(
        output.contains("pub mod physics {"),
        "Should have nested module"
    );
    assert!(
        output.contains("pub struct World"),
        "Should have struct in nested module"
    );
}
