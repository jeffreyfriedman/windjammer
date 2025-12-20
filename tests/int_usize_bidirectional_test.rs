//! TDD: Bidirectional int/usize comparison auto-casting
//!
//! BUG: Comparing i64 with usize in both directions causes type errors
//!
//! Example 1: usize >= i64
//! ```windjammer
//! let index: usize = 5;
//! let limit: int = 10;
//! if index >= limit {  // ❌ E0308: expected usize, found i64
//! }
//! ```
//!
//! Example 2: i64 >= usize  
//! ```windjammer
//! let count: int = 5;
//! let max: usize = 10;
//! if count >= max {  // ❌ E0308: expected i64, found usize
//! }
//! ```
//!
//! EXPECTED: Auto-cast to common type (i64)

use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::CompilationTarget;

fn parse_and_generate(code: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_usize_compared_to_i64() {
    let source = r#"
struct Query {
    index: usize,
    limit: int
}

impl Query {
    fn is_done(self) -> bool {
        if self.index >= self.limit {
            return true
        }
        return false
    }
}
"#;

    let output = parse_and_generate(source);
    
    println!("Generated Rust:\n{}", output);
    
    // Should cast usize to i64 for comparison
    assert!(
        output.contains("self.index as i64") || output.contains("(self.index as i64)"),
        "Expected 'self.index as i64' for usize->i64 cast, got:\n{}",
        output
    );
}

#[test]
fn test_i64_compared_to_usize() {
    let source = r#"
struct Counter {
    count: int,
    max: usize
}

impl Counter {
    fn is_full(self) -> bool {
        if self.count >= self.max {
            return true
        }
        return false
    }
}
"#;

    let output = parse_and_generate(source);
    
    println!("Generated Rust:\n{}", output);
    
    // Should cast usize to i64 for comparison
    assert!(
        output.contains("self.max as i64") || output.contains("(self.max as i64)"),
        "Expected 'self.max as i64' for usize->i64 cast, got:\n{}",
        output
    );
}

#[test]
fn test_vec_len_compared_to_usize_field() {
    let source = r#"
struct Buffer {
    data: Vec<int>,
    capacity: usize
}

impl Buffer {
    fn is_full(self) -> bool {
        if self.data.len() >= self.capacity {
            return true
        }
        return false
    }
}
"#;

    let output = parse_and_generate(source);
    
    println!("Generated Rust:\n{}", output);
    
    // Vec::len() returns usize, capacity is usize, no cast needed
    assert!(
        !output.contains("as i64"),
        "No cast needed for usize >= usize comparison, but found cast in:\n{}",
        output
    );
}

