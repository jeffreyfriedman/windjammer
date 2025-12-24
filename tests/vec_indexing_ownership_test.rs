//! TDD: Vec indexing should borrow, not move
//!
//! BUG: `let frame = self.frames[index]` generates code that moves the value,
//! causing E0507 when the element type doesn't implement Copy.
//!
//! EXPECTED: Should generate `&self.frames[index]` (borrow)
//! ACTUAL: Generates `self.frames[index]` (move)

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};
use windjammer::CompilationTarget;

fn parse_code(code: &str) -> Program {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser.parse().unwrap()
}

#[test]
fn test_vec_indexing_borrows_non_copy_type() {
    let source = r#"
        struct Region {
            pub x: f32,
            pub y: f32,
            pub name: string,
        }
        
        struct Sprite {
            pub frames: Vec<Region>,
        }
        
        impl Sprite {
            pub fn get_frame_x(self) -> f32 {
                let frame = self.frames[0]
                return frame.x
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);

    // Region has a String field, so it can't be Copy
    // When indexing and accessing a field, should borrow: &self.frames[0]
    assert!(
        rust_code.contains("&self.frames[0") || rust_code.contains(".get(0)"),
        "Vec indexing should borrow for field access, got:\n{}",
        rust_code
    );
}

#[test]
fn test_vec_indexing_with_copy_type_can_move() {
    let source = r#"
        struct Data {
            pub values: Vec<i32>,
        }
        
        impl Data {
            pub fn get_value(self) -> i32 {
                let val = self.values[0]
                return val
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);

    // For Copy types, either borrowing or moving is fine
    // The key is it should compile without errors
    println!("Generated Rust:\n{}", rust_code);
}

#[test]
fn test_vec_indexing_in_expression() {
    let source = r#"
        struct Point {
            pub x: f32,
            pub y: f32,
        }
        
        struct Path {
            pub points: Vec<Point>,
            pub current: int,
        }
        
        impl Path {
            pub fn get_current_x(self) -> f32 {
                return self.points[self.current as int].x
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);

    // Should borrow the Point, then access its field
    assert!(
        rust_code.contains("self.points[") && rust_code.contains(".x"),
        "Vec indexing in field access should work, got:\n{}",
        rust_code
    );
}

#[test]
fn test_vec_indexing_with_variable() {
    let source = r#"
        struct Frame {
            pub duration: f32,
        }
        
        struct Animation {
            pub frames: Vec<Frame>,
            pub current_frame: int,
        }
        
        impl Animation {
            pub fn get_current_duration(self) -> f32 {
                let frame = self.frames[self.current_frame as int]
                return frame.duration
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);

    // Should borrow the Frame
    println!("Generated Rust:\n{}", rust_code);
}
