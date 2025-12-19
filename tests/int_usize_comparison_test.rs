//! TDD: int vs usize comparison type inference
//!
//! BUG: Comparing int (i64) with usize causes E0308
//!
//! Example:
//! ```windjammer
//! if self.index >= self.entities.len() {
//!     // self.index is i64, len() returns usize
//! }
//! ```
//!
//! EXPECTED: Auto-cast to make comparison work
//! ACTUAL: E0308 mismatched types

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
fn test_int_vs_usize_comparison() {
    let source = r#"
        struct Iterator {
            pub index: int,
            pub items: Vec<int>,
        }
        
        impl Iterator {
            pub fn has_next(self) -> bool {
                return self.index < self.items.len()
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);
    
    // Should cast one side to match the other
    assert!(
        rust_code.contains("as i64") || rust_code.contains("as usize"),
        "int vs usize comparison should have auto-cast, got:\n{}",
        rust_code
    );
}

#[test]
fn test_int_vs_usize_greater_than_or_equal() {
    let source = r#"
        struct World {
            pub index: int,
            pub entities: Vec<int>,
        }
        
        impl World {
            pub fn next(self) -> bool {
                if self.index >= self.entities.len() {
                    return false
                }
                return true
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);
    
    // Should cast .len() to i64 or index to usize
    assert!(
        rust_code.contains(".len() as i64") || rust_code.contains("self.index as usize"),
        "int vs usize comparison should have auto-cast, got:\n{}",
        rust_code
    );
}

