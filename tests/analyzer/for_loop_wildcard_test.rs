// TDD Test: For-loop wildcard pattern `for _ in 0..3 { ... }`
//
// Bug: The for-loop parser only handled Ident, LParen, and Ampersand tokens
// as valid loop variable patterns. Token::Underscore (wildcard `_`) was missing,
// causing `for _ in 0..3 { ... }` to fail with:
//   "Expected variable name, reference pattern, or tuple pattern in for loop"
//
// Root Cause: The parse_for() function had explicit checks for &, (, and Ident
// but forgot to handle Token::Underscore. The general parse_pattern() function
// DOES handle wildcards, but wasn't being called for the Underscore case.
//
// Fix: Use parse_pattern() for all non-reference patterns in for-loops,
// which handles identifiers, wildcards, tuples, and all other pattern types.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_for_wildcard_pattern() {
    // THE BUG: `for _ in 0..3` fails to parse
    let (generated, ok) = test_utils::compile_single_check(
        r#"
fn main() {
    for _ in 0..3 {
        println("hello")
    }
}
"#,
    );

    assert!(
        ok,
        "for _ in 0..3 should parse successfully.\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("for _ in"),
        "Generated Rust should contain `for _ in`.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_for_wildcard_nested() {
    // Nested for-loops with wildcard outer loop (the exact failing pattern from puzzle_game.wj)
    let (generated, ok) = test_utils::compile_single_check(
        r#"
fn main() {
    for _ in 0..3 {
        for col in 0..4 {
            println("{}", col)
        }
    }
}
"#,
    );

    assert!(
        ok,
        "Nested for with wildcard should parse.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_for_tuple_destructure() {
    // Tuple destructuring in for-loop: for (i, item) in items.iter().enumerate()
    let (generated, ok) = test_utils::compile_single_check(
        r#"
fn main() {
    let items = vec![10, 20, 30]
    for (i, item) in items.iter().enumerate() {
        println("index {}: {}", i, item)
    }
}
"#,
    );

    assert!(
        ok,
        "for (i, item) in ... should parse.\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("(i, item)") || generated.contains("(_i, _item)"),
        "Generated Rust should contain tuple destructuring.\nGenerated:\n{}",
        generated
    );
}
