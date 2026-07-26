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

//! TDD: `for i in items { *i }` must borrow the Vec formal (`items: &Vec<T>`),
//! and bare identifiers in for-iterables / index expressions must not be treated
//! as call-argument forward-ref uses.

#[path = "common/test_utils.rs"]
mod test_utils;

use windjammer::analyzer::{Analyzer, OwnershipMode};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

#[test]
fn test_for_loop_element_deref_infers_borrowed_vec() {
    let code = r#"
pub fn process_items(items: Vec<i32>) -> i32 {
    let mut sum = 0
    for i in items {
        sum = sum + *i
    }
    sum
}
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().expect("parse");
    let mut analyzer = Analyzer::new();
    let (funcs, _, _) = analyzer.analyze_program(&program).expect("analyze");
    let func = funcs
        .iter()
        .find(|f| f.decl.name == "process_items")
        .expect("fn");
    assert_eq!(
        func.inferred_ownership.get("items").copied(),
        Some(OwnershipMode::Borrowed),
        "for i in items {{ *i }} should borrow the Vec"
    );
}

#[test]
fn test_for_loop_element_deref_emits_borrowed_formal_and_call_site() {
    let code = r#"
pub fn process_items(items: Vec<i32>) -> i32 {
    let mut sum = 0
    for i in items {
        sum = sum + *i
    }
    sum
}

pub fn main() -> i32 {
    let v = vec![1, 2, 3]
    process_items(v)
}
"#;
    let (generated, success) = test_utils::compile_single_check(code);
    assert!(
        generated.contains("items: &Vec<i32>") || generated.contains("items: &[i32]"),
        "formal should be borrowed Vec. Got:\n{generated}"
    );
    assert!(
        generated.contains("process_items(&v)"),
        "call site should auto-borrow. Got:\n{generated}"
    );
    assert!(success, "must compile. Generated:\n{generated}");
}
