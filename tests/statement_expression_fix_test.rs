//! TDD Test: Statement expressions should NOT be borrowed
//!
//! Bug: When vec.push(item) is in an if block or match arm, the compiler incorrectly
//! generates &mut vec.push(item), producing &mut () instead of ().
//!
//! Root Cause: Method call object gets &mut prefix, but format! produces
//! "&mut obj.push(args)" which parses as &mut (obj.push(args)) due to operator precedence.
//!
//! Fix: Wrap object in parentheses when it starts with & or &mut for instance method calls:
//! (&mut obj).push(args) instead of &mut obj.push(args)

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_push_in_if_block_no_borrow() {
    // vec.push(item) in if block - should NOT generate &mut vec.push(item)
    let source = r#"
pub fn collect_items() -> Vec<i32> {
    let mut items: Vec<i32> = Vec::new()
    if true {
        items.push(1)
    }
    items
}
"#;
    let (result, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "Should compile. Generated:\n{}\n\nrustc error above",
        result
    );
    // Must NOT have &mut items.push - that produces &mut ()
    assert!(
        !result.contains("&mut items.push("),
        "Should NOT add &mut to statement expression. Got:\n{}",
        result
    );
    // Integer literals are often suffixed (_i32) in generated Rust
    assert!(
        result.contains("items.push(1") || result.contains("(&mut items).push(1)"),
        "Should have valid push call. Got:\n{}",
        result
    );
}

#[test]
fn test_vec_push_in_match_arm_no_borrow() {
    // vec.push(item) in match arm - statement position
    let source = r#"
pub fn process(x: i32) -> Vec<i32> {
    let mut result: Vec<i32> = Vec::new()
    match x {
        0 => result.push(0),
        1 => result.push(1),
        _ => result.push(-1),
    }
    result
}
"#;
    let (result, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "Should compile. Generated:\n{}\n\nrustc error above",
        result
    );
    assert!(
        !result.contains("&mut result.push("),
        "Should NOT add &mut to statement expression in match arm. Got:\n{}",
        result
    );
}

#[test]
fn test_vec_push_other_unit_methods_in_statement_position() {
    // Other ()-returning methods: clear(), etc.
    let source = r#"
pub fn clear_vec() {
    let mut v: Vec<i32> = Vec::new()
    v.push(1)
    v.clear()
}
"#;
    let (result, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "Should compile. Generated:\n{}\n\nrustc error above",
        result
    );
    assert!(
        !result.contains("&mut v.push("),
        "Should NOT add &mut to v.push(). Got:\n{}",
        result
    );
    assert!(
        !result.contains("&mut v.clear("),
        "Should NOT add &mut to v.clear(). Got:\n{}",
        result
    );
}
