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

//! TDD Test: Fix HashMap.get() double-reference bug
//!
//! Bug: When calling HashMap.get() with an already-borrowed key from .keys(),
//! the transpiler incorrectly adds another `&`, resulting in `&&String` instead of `&String`.
//!
//! Example:
//!   Windjammer: `map.get(key)` where key is `&String`
//!   Generated:  `map.get(&key)` which is `&&String` ❌
//!   Should be:  `map.get(key)` which is `&String` ✅

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_with_borrowed_key_from_keys() {
    // Test Case 1: HashMap.get() with key from .keys() iterator
    let code = r#"
use std::collections::HashMap

pub fn get_values(map: &HashMap<string, i32>) -> Vec<i32> {
    let mut result = Vec::new()
    
    for key in map.keys() {
        let value = map.get(key)
        match value {
            Some(v) => result.push(*v),
            None => {}
        }
    }
    
    result
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // Should NOT contain map.get(&key) which causes double-ref
    assert!(
        !generated.contains("map.get(&key)"),
        "Generated code should not double-reference key: map.get(&key) is wrong"
    );

    // Should contain map.get(key)
    assert!(
        generated.contains("map.get(key)"),
        "Generated code should use single reference: map.get(key)"
    );

    assert!(
        success,
        "Generated Rust code should compile successfully. Rustc error:\n{}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_with_owned_key() {
    // Test Case 2: HashMap.get() with owned String key
    let code = r#"
use std::collections::HashMap

pub fn lookup(map: &HashMap<string, i32>, key: string) -> Option<i32> {
    match map.get(key) {
        Some(v) => Some(*v),
        None => None
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // When key is owned String, should auto-borrow to &String
    assert!(
        generated.contains("map.get(&key)") || generated.contains("map.get(key)"),
        "Generated code should handle owned String key correctly"
    );

    assert!(
        success,
        "Generated Rust code should compile successfully. Rustc error:\n{}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_with_explicit_borrowed_key() {
    // Test Case 3: HashMap.get() with already-borrowed key parameter
    let code = r#"
use std::collections::HashMap

pub fn lookup_borrowed(map: &HashMap<string, i32>, key: &string) -> Option<i32> {
    match map.get(key) {
        Some(v) => Some(*v),
        None => None
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // When key is already &String, should NOT add another &
    assert!(
        !generated.contains("map.get(&key)"),
        "Should not double-reference when key is already &String"
    );

    assert!(
        generated.contains("map.get(key)"),
        "Should use key directly when it's already borrowed"
    );

    assert!(
        success,
        "Generated Rust code should compile successfully. Rustc error:\n{}",
        err
    );
}
