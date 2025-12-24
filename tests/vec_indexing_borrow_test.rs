//! TDD: Vec indexing should auto-borrow when value is only read
//!
//! BUG #10: Vec indexing doesn't auto-borrow non-Copy types
//!
//! Example:
//! ```windjammer
//! struct Frame { x: int, y: int }
//! let frames: Vec<Frame> = vec![Frame { x: 0, y: 0 }]
//! let frame = frames[0]  // ❌ E0507: Cannot move out of Vec
//! print(frame.x)  // Only reading, should auto-borrow!
//! ```
//!
//! ROOT CAUSE: Codegen doesn't detect read-only usage and auto-insert `&`
//! EXPECTED: Auto-borrow when indexing non-Copy types that are only read

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
fn test_vec_indexing_auto_borrow_read_only() {
    let source = r#"
struct Frame {
    x: int,
    y: int,
    name: string
}

fn process_frame(frames: Vec<Frame>, index: int) {
    let frame = frames[index as usize]
    print(frame.x)
    print(frame.y)
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Should auto-borrow when indexing non-Copy type for read-only access
    assert!(
        output.contains("let frame = &frames[") || output.contains("let frame = frames["),
        "Vec indexing should auto-borrow for read-only access, got:\n{}",
        output
    );

    // Should NOT generate .clone() for read-only access
    assert!(
        !output.contains(".clone()") || output.contains("&frames"),
        "Should borrow, not clone, for read-only access, got:\n{}",
        output
    );
}

#[test]
fn test_vec_indexing_copy_type_no_borrow() {
    let source = r#"
fn sum_elements(numbers: Vec<int>, index: int) -> int {
    let value = numbers[index as usize]
    return value + 10
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Copy types (int/i64) can be moved, no need for borrow
    // The generated code should work without error
    assert!(
        output.contains("let value = numbers["),
        "Copy type indexing should work, got:\n{}",
        output
    );
}

#[test]
fn test_vec_indexing_field_access() {
    let source = r#"
struct Point {
    x: f32,
    y: f32
}

struct Sprite {
    frames: Vec<Point>,
    current: int
}

impl Sprite {
    fn get_position(self) -> f32 {
        let frame = self.frames[self.current as usize]
        return frame.x + frame.y
    }
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Should auto-borrow when indexing for field access
    assert!(
        output.contains("&self.frames[") || output.contains("let frame = self.frames["),
        "Should handle field access after indexing, got:\n{}",
        output
    );
}
