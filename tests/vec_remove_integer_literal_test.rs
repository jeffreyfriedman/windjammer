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

//! TDD Test: Fix Vec.remove() integer literal auto-ref bug
//!
//! Bug: When calling Vec.remove() with an integer literal,
//! the transpiler incorrectly adds `&`, resulting in `&{integer}` instead of `usize`.
//!
//! Example:
//!   Windjammer: `vec.remove(0)`
//!   Generated:  `vec.remove(&0)` ❌ Type error: expected usize, found &{integer}
//!   Should be:  `vec.remove(0)` ✅

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_remove_integer_literal() {
    let code = r#"
pub fn remove_first<T: Clone>(vec: &mut Vec<T>) -> Option<T> {
    if vec.len() > 0 {
        Some(vec.remove(0))
    } else {
        None
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        !generated.contains("vec.remove(&0)"),
        "Should not auto-ref integer literal: vec.remove(&0) is wrong"
    );

    assert!(
        generated.contains("vec.remove(0)"),
        "Should use integer literal directly: vec.remove(0)"
    );

    assert!(
        success,
        "Generated Rust code should compile successfully. Rustc error:\n{}",
        err
    );
}
