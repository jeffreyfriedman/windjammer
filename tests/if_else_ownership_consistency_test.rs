//! TDD: if/else branches must have consistent ownership
//!
//! BUG: If one branch returns owned value, other branch must too
//!
//! Example:
//! ```windjammer
//! let result = if condition {
//!     MyStruct::new()  // owned
//! } else {
//!     existing_value   // borrowed
//! }
//! ```
//!
//! EXPECTED: Both branches return owned or both return borrowed
//! ACTUAL: E0308 - mismatched types

use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};
use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::CompilationTarget;

fn parse_code(code: &str) -> Program {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser.parse().unwrap()
}

#[test]
fn test_if_else_return_consistency() {
    let source = r#"
        struct Region {
            pub x: f32,
            pub y: f32,
            pub name: string,
        }
        
        impl Region {
            pub fn new(x: f32, y: f32, name: string) -> Region {
                return Region { x: x, y: y, name: name }
            }
            
            pub fn adjust(self, other: Region, negative: bool) -> Region {
                let dot = self.x * other.x + self.y * other.y
                
                let other_adjusted = if negative {
                    Region::new(-other.x, -other.y, "negated".to_string())
                } else {
                    other
                }
                
                return other_adjusted
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Parameter 'other' is used in field access (other.x, other.y)
    // AND returned in the else branch
    // The analyzer should infer 'other: Region' (owned), not '&Region'
    // OR add .clone() in the else branch
    assert!(
        rust_code.contains("other: Region") || rust_code.contains("other.clone()"),
        "Parameter used in if/else should be owned when returned, got:\n{}",
        rust_code
    );
}

#[test]
fn test_if_else_direct_return() {
    let source = r#"
        struct Vec2 {
            pub x: f32,
            pub y: f32,
        }
        
        impl Vec2 {
            pub fn select(self, other: Vec2, use_other: bool) -> Vec2 {
                if use_other {
                    return other
                } else {
                    return self
                }
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);
    
    // Both parameters should be owned (not borrowed)
    assert!(
        rust_code.contains("other: Vec2") || rust_code.contains("other: Self"),
        "Parameter returned in if should be owned, got:\n{}",
        rust_code
    );
}

