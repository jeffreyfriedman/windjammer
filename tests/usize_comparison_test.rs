//! TDD: usize comparisons should not cast to i64
//!
//! BUG #13: Comparing usize values incorrectly casts to i64
//!
//! Example:
//! ```windjammer
//! if vec.len() >= max as usize {  // ❌ Generates: (vec.len() as i64) >= max as usize
//! }
//! ```
//!
//! ROOT CAUSE: Auto-cast logic casts .len() to i64 even when comparing with usize
//! EXPECTED: Keep both sides as usize

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate(code: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_usize_comparison_no_i64_cast() {
    let source = r#"
struct Buffer {
    items: Vec<int>,
    max_size: int
}

impl Buffer {
    fn is_full(self) -> bool {
        return self.items.len() >= self.max_size as usize
    }
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Should not cast .len() to i64 when comparing with usize
    // The comparison line should have .len() as usize on the left
    assert!(
        !output.contains(".len() as i64"),
        "Should not cast .len() to i64 when comparing with usize, got:\n{}",
        output
    );

    // Both sides should be usize
    assert!(
        output.contains("self.items.len() >= (self.max_size as usize)")
            || output.contains("self.items.len() >= self.max_size as usize"),
        "Comparison should keep both sides as usize, got:\n{}",
        output
    );
}

#[test]
fn test_usize_comparison_with_literal() {
    let source = r#"
fn check_size(items: Vec<string>) -> bool {
    return items.len() >= 10
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Should not cast .len() to i64 when comparing with literal
    assert!(
        !output.contains(".len() as i64"),
        "Should not cast .len() to i64, got:\n{}",
        output
    );
}

#[test]
fn test_usize_comparison_both_len() {
    let source = r#"
fn compare_sizes(a: Vec<int>, b: Vec<int>) -> bool {
    return a.len() >= b.len()
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Should not cast .len() to i64 when both sides are .len()
    assert!(
        !output.contains(".len() as i64"),
        "Should not cast .len() to i64 when both sides are usize, got:\n{}",
        output
    );

    // Should be simple comparison
    assert!(
        output.contains("a.len() >= b.len()"),
        "Should be simple usize comparison, got:\n{}",
        output
    );
}
