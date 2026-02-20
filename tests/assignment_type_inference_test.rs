//! TDD: Assignment should infer correct type from target variable
//!
//! BUG #12: Assignment casts to wrong type
//!
//! Example:
//! ```windjammer
//! struct Query { index: usize }
//! impl Query {
//!     fn next(self) {
//!         self.index = self.index + 1  // ❌ Generates: as i32 instead of as usize
//!     }
//! }
//! ```
//!
//! ROOT CAUSE: Codegen doesn't check target variable type when generating casts
//! EXPECTED: Use target variable's type for the cast

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
fn test_assignment_usize_field() {
    let source = r#"
struct Counter {
    index: usize
}

impl Counter {
    fn increment(self) {
        self.index = self.index + 1
    }
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Should not cast to i32 when target is usize
    assert!(
        !output.contains("as i32"),
        "Should not cast to i32 when target field is usize, got:\n{}",
        output
    );

    // Should either have no cast or cast to usize
    // The ideal is no cast at all for usize + int literal
    // Also accept compound assignment optimization: self.index += 1
    assert!(
        output.contains("self.index = self.index + 1")
            || output.contains("self.index + 1usize")
            || output.contains("self.index += 1")
            || output.contains("as usize"),
        "Assignment to usize field should not cast to i32, got:\n{}",
        output
    );
}

#[test]
fn test_assignment_int_field() {
    let source = r#"
struct Counter {
    count: int
}

impl Counter {
    fn increment(self) {
        self.count = self.count + 1
    }
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // For int (i64) field, should cast to i64 or no cast
    assert!(
        !output.contains("as usize"),
        "Should not cast to usize when target field is i64, got:\n{}",
        output
    );
}

#[test]
fn test_assignment_with_complex_expression() {
    let source = r#"
struct Position {
    x: usize,
    y: int
}

impl Position {
    fn update(self) {
        self.x = self.x + 10
        self.y = self.y + 20
    }
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // x is usize, should not cast to i32
    let x_assignment = output
        .lines()
        .find(|line| line.contains("self.x ="))
        .unwrap_or("");

    assert!(
        !x_assignment.contains("as i32"),
        "usize field assignment should not cast to i32, got: {}",
        x_assignment
    );

    // y is i64, should not cast to usize
    let y_assignment = output
        .lines()
        .find(|line| line.contains("self.y ="))
        .unwrap_or("");

    assert!(
        !y_assignment.contains("as usize"),
        "i64 field assignment should not cast to usize, got: {}",
        y_assignment
    );
}
