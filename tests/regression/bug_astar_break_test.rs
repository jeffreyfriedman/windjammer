#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// BUG: Parser reports "Unexpected Break token" in match arm
//
// DISCOVERED DURING: Dogfooding astar_grid.wj
//
// PROBLEM:
// Windjammer source: `match x { Some(v) => {...}, None => break }`
// Parser fails: "Unexpected token in expression: Break (at token position N)"
//
// ROOT CAUSE:
// Match arm body without braces is parsed as expression. Expression parser
// does not handle break/continue/return (control-flow statements).
//
// FIX:
// When match arm body starts with Break, Continue, or Return, parse as
// statement and wrap in block (same as assignment handling).

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_break_in_match_arm() {
    // Minimal: loop with match, None => break (from astar_grid.wj path reconstruction)
    let source = r#"
fn test() {
    let mut x = 0
    loop {
        match x {
            0 => { x = 1 },
            1 => break,
        }
    }
}
"#;

    let result = test_utils::compile_single_result(source);
    assert!(
        result.is_ok(),
        "break in match arm should compile. Error: {:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("break"),
        "Generated Rust should contain break"
    );
}

#[test]
fn test_continue_in_match_arm() {
    let source = r#"
fn test() {
    let mut i = 0
    while i < 10 {
        match i % 2 {
            0 => continue,
            _ => { i = i + 1 },
        }
        i = i + 1
    }
}
"#;

    let result = test_utils::compile_single_result(source);
    assert!(
        result.is_ok(),
        "continue in match arm should compile. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_return_in_match_arm() {
    let source = r#"
fn test(x: i32) -> i32 {
    match x {
        0 => return 0,
        _ => x + 1,
    }
}
"#;

    let result = test_utils::compile_single_result(source);
    assert!(
        result.is_ok(),
        "return in match arm should compile. Error: {:?}",
        result.err()
    );
}
