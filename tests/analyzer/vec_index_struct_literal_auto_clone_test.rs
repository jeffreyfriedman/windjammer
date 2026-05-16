#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: Vec index used in struct literal should auto-clone
//!
//! Bug: When a variable is bound from vec[i] and then used in a struct literal
//! field AND also used afterwards (e.g. for iteration), the compiler generates
//! a borrow (&vec[i]) because analyze_variable_usage_in_expression doesn't
//! recognize StructLiteral as a "move" site. The user was forced to write
//! .clone() in WJ source, which is Rust leakage.
//!
//! Fix: The compiler should auto-detect that the variable is both moved into
//! a struct literal and used afterwards, and automatically emit .clone().

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_vec_index_moved_into_struct_and_used_after() {
    // Pattern: let group = items[i]
    //          SomeStruct { field: group }  // move
    //          group.len()                  // use after move
    // Compiler must auto-clone, user should NOT write .clone()
    let source = r#"
pub struct Step {
    pub indices: Vec<usize>,
    pub active: bool,
}

pub fn build_steps(groups: Vec<Vec<usize>>) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new()
    let mut i: usize = 0
    while i < groups.len() {
        let group = groups[i]
        let count = group.len()
        steps.push(Step { indices: group, active: count > 0 })
        i = i + 1
    }
    steps
}
"#;

    let output = parse_and_generate(source);

    // The generated Rust must compile -- group must be cloned, not borrowed
    // It should NOT contain `&groups[` for the let binding (that would fail
    // because group is moved into Step AND used for .len())
    assert!(
        !output.contains("&groups["),
        "Generated code borrows from vec index but value is moved into struct.\n\
         Compiler should auto-clone.\nGenerated:\n{}",
        output
    );

    // Should contain a clone somewhere for the vec indexing
    assert!(
        output.contains(".clone()"),
        "Generated code must clone vec element that is both moved into struct and used after.\n\
         Generated:\n{}",
        output
    );
}

#[test]
fn test_vec_index_only_moved_into_struct_no_later_use() {
    // Pattern: let group = items[i]
    //          SomeStruct { field: group }  // move only, no later use
    // Compiler can either clone or move -- both valid
    let source = r#"
pub struct Step {
    pub indices: Vec<usize>,
}

pub fn build_steps(groups: Vec<Vec<usize>>) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new()
    let mut i: usize = 0
    while i < groups.len() {
        let group = groups[i]
        steps.push(Step { indices: group })
        i = i + 1
    }
    steps
}
"#;

    let output = parse_and_generate(source);

    // Must not generate &groups[...] without a clone, since group is moved
    // into the struct literal
    assert!(
        !output.contains("&groups[") || output.contains(".clone()"),
        "Generated code borrows from vec but moves into struct -- must clone or own.\n\
         Generated:\n{}",
        output
    );
}

#[test]
fn test_vec_index_used_in_struct_field_and_method_call() {
    // group used for group[0], group.len(), AND moved into struct
    let source = r#"
pub struct Step {
    pub indices: Vec<usize>,
    pub first: usize,
    pub count: usize,
}

pub fn build(groups: Vec<Vec<usize>>) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new()
    let mut i: usize = 0
    while i < groups.len() {
        let group = groups[i]
        let first = group[0]
        let count = group.len()
        steps.push(Step { indices: group, first: first, count: count })
        i = i + 1
    }
    steps
}
"#;

    let output = parse_and_generate(source);

    assert!(
        !output.contains("&groups[") || output.contains(".clone()"),
        "Vec index into struct literal + usage requires clone.\nGenerated:\n{}",
        output
    );
}
