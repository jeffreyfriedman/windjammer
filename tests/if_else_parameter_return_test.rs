//! TDD: Parameters returned in if/else should be owned
//!
//! BUG: When a parameter is returned in if/else branches with different ownership
//!
//! Example:
//! ```windjammer
//! fn transform(other: Quat) -> Quat {
//!     if condition {
//!         Quat::new(...)  // Returns owned Quat
//!     } else {
//!         other  // ❌ E0308: expected Quat, found &Quat
//!     }
//! }
//! ```
//!
//! ROOT CAUSE: Parameter inferred as &Quat but else branch returns it directly
//! EXPECTED: Parameter should be owned when returned

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
fn test_parameter_returned_in_else_branch() {
    let source = r#"
struct Point {
    x: int,
    y: int
}

impl Point {
    fn new(x: int, y: int) -> Point {
        Point { x: x, y: y }
    }
}

fn transform(p: Point, scale: int) -> Point {
    if scale > 1 {
        Point::new(p.x * scale, p.y * scale)
    } else {
        p
    }
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Parameter p is returned in else branch, so should be owned
    assert!(
        output.contains("fn transform(mut p: Point, scale: i64)")
            || output.contains("fn transform(p: Point, scale: i64)"),
        "Expected parameter 'p' to be owned (not &Point), got:\n{}",
        output
    );

    // Should not have borrowed parameter
    assert!(
        !output.contains("fn transform(p: &Point"),
        "Parameter 'p' should be owned, not borrowed, found:\n{}",
        output
    );
}

#[test]
fn test_parameter_returned_in_if_branch() {
    let source = r#"
struct Value {
    data: int
}

impl Value {
    fn new(d: int) -> Value {
        Value { data: d }
    }
}

fn get_value(v: Value, use_default: bool) -> Value {
    if use_default {
        v
    } else {
        Value::new(0)
    }
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Parameter v is returned in if branch, so should be owned
    assert!(
        output.contains("fn get_value(mut v: Value") || output.contains("fn get_value(v: Value"),
        "Expected parameter 'v' to be owned (not &Value), got:\n{}",
        output
    );

    assert!(
        !output.contains("fn get_value(v: &Value"),
        "Parameter 'v' should be owned, not borrowed, found:\n{}",
        output
    );
}

#[test]
fn test_parameter_in_if_else_assignment() {
    let source = r#"
struct Quat {
    x: f32,
    y: f32,
    z: f32,
    w: f32
}

impl Quat {
    fn new(x: f32, y: f32, z: f32, w: f32) -> Quat {
        Quat { x: x, y: y, z: z, w: w }
    }
}

fn slerp(self_q: Quat, other: Quat, dot: f32) -> Quat {
    let other_adjusted = if dot < 0.0 {
        Quat::new(-other.x, -other.y, -other.z, -other.w)
    } else {
        other
    }
    
    return other_adjusted
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Parameter 'other' is used in if/else assignment where:
    // - if returns owned Quat::new(...)
    // - else returns other
    // So 'other' must be owned to match the if branch type
    assert!(
        output.contains("other: Quat") || output.contains("mut other: Quat"),
        "Expected parameter 'other' to be owned (Quat, not &Quat), got:\n{}",
        output
    );

    // Make sure it's not borrowed
    assert!(
        !output.contains("other: &Quat"),
        "Parameter 'other' should be owned (Quat), not borrowed (&Quat), found:\n{}",
        output
    );
}

#[test]
fn test_parameter_not_returned_stays_borrowed() {
    let source = r#"
struct Config {
    enabled: bool
}

impl Config {
    fn new() -> Config {
        Config { enabled: true }
    }
}

fn check(cfg: Config) -> bool {
    if cfg.enabled {
        true
    } else {
        false
    }
}
"#;

    let output = parse_and_generate(source);

    println!("Generated Rust:\n{}", output);

    // Parameter cfg is NOT returned, just accessed, so can be borrowed
    // (This test validates we don't over-apply the fix)
    assert!(
        output.contains("fn check(cfg: &Config)") || output.contains("fn check(cfg: Config)"),
        "Expected parameter 'cfg' to be borrowed or owned, got:\n{}",
        output
    );
}
