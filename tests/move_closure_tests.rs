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

//! Tests for smart closure move inference.
//!
//! Windjammer Philosophy: The compiler does the work, not the developer.
//! The compiler adds `move` when a closure captures outer variables.
//! Pure closures (no captures) don't get `move`.
//! The user never writes `move` explicitly.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closures_that_capture_get_move() {
    let generated = test_utils::compile_fixture("move_closures").expect("Compilation failed");

    // Closures that capture outer variables should get `move`
    assert!(
        generated.contains("let closure = move || x + 1"),
        "Closure capturing outer `x` should get `move`. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("let add_offset = move |n| n + offset"),
        "Closure capturing outer `offset` should get `move`. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_pure_closures_no_move() {
    let generated = test_utils::compile_fixture("move_closures").expect("Compilation failed");

    // Pure closures (only use their own params, no captures) should NOT get `move`
    assert!(
        generated.contains("let double = |x| x * 2"),
        "Pure closure (no captures) should not get `move`. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_thread_blocks_always_move() {
    let generated = test_utils::compile_fixture("move_closures").expect("Compilation failed");

    assert!(
        generated.contains("std::thread::spawn(move ||"),
        "Thread blocks should always use `move`. Generated:\n{}",
        generated
    );
}
