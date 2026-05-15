//! TDD tests for the Map<K, V> type in Windjammer stdlib.
//!
//! Bug: No Map type exists in Windjammer - users must fall back to
//!      Rust's std::collections::HashMap which is backend-specific.
//! Root Cause: Missing stdlib type for key-value storage.
//! Fix: Add Map<K, V> type that compiles to HashMap (Rust), map (Go),
//!      Map (JS), etc.

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
fn test_map_type_generates_hashmap() {
    let source = r#"
    fn create_scores() -> Map<String, i32> {
        let mut scores = Map::new()
        scores.insert("Alice", 100)
        scores
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("HashMap") || output.contains("std::collections::HashMap"),
        "Map<K,V> should generate HashMap in Rust. Got:\n{}",
        output
    );
}

#[test]
fn test_map_variable_declaration() {
    let source = r#"
    fn use_map() {
        let mut m: Map<String, i32> = Map::new()
        m.insert("key", 42)
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("HashMap<String, i32>"),
        "Map type annotation should map to HashMap. Got:\n{}",
        output
    );
}

#[test]
fn test_map_new_generates_hashmap_new() {
    let source = r#"
    fn make_map() -> Map<String, String> {
        Map::new()
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("HashMap::new()") || output.contains("HashMap::<String, String>::new()"),
        "Map::new() should generate HashMap::new(). Got:\n{}",
        output
    );
}

#[test]
fn test_map_in_struct_field() {
    let source = r#"
    struct PlayerStats {
        scores: Map<String, i32>,
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("HashMap<String, i32>"),
        "Map in struct field should be HashMap. Got:\n{}",
        output
    );
}

#[test]
fn test_map_as_parameter() {
    let source = r#"
    fn process(data: Map<String, f32>) -> i32 {
        data.len()
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("HashMap<String, f32>"),
        "Map as parameter should be HashMap. Got:\n{}",
        output
    );
}
