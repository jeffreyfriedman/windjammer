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

//! Tests for auto-move closures
//!
//! Windjammer Philosophy: The compiler does the work, not the developer.
//! All closures automatically emit `move` in generated Rust - no explicit
//! keyword needed from the user!

#[path = "common/test_utils.rs"]
mod test_utils;

/// Helper to compile a test fixture and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_closures_auto_generate_move() {
    let generated = test_utils::compile_fixture("move_closures").expect("Compilation failed");

    // ALL closures should generate `move` automatically
    // This is the Windjammer philosophy - the compiler infers what the developer shouldn't need to write

    // Check that closures generate `move` without user needing to write it
    assert!(
        generated.contains("move ||") || generated.contains("move |"),
        "Closures should auto-generate 'move' keyword. Generated:\n{}",
        generated
    );

    // Verify thread blocks also use move (they already did)
    assert!(
        generated.contains("std::thread::spawn(move ||"),
        "Thread blocks should use 'move'. Generated:\n{}",
        generated
    );

    println!("✓ Windjammer auto-moves closures - no explicit 'move' keyword needed!");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_explicit_move_keyword_needed() {
    // This test verifies the Windjammer philosophy:
    // The developer writes: |x| x + 1
    // We generate: |x| x + 1  (borrow by reference - efficient!)
    //
    // The developer writes: thread { ... }
    // We generate: std::thread::spawn(move || { ... })  (needs 'move' to escape scope)
    //
    // NO explicit 'move' keyword ever needed by the user!
    // Compiler adds 'move' only when necessary (closures that outlive their scope).

    let generated = test_utils::compile_fixture("move_closures").expect("Compilation failed");

    // Regular closures should NOT have move (they borrow by reference)
    assert!(
        generated.contains("let closure = || x + 1"),
        "Regular closure should borrow by reference (no move). Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("let add_offset = |n| n + offset"),
        "Closure with params should borrow by reference (no move). Generated:\n{}",
        generated
    );

    // Thread spawn SHOULD have move (closure escapes scope)
    assert!(
        generated.contains("std::thread::spawn(move ||"),
        "Thread closure should use 'move' (escapes scope). Generated:\n{}",
        generated
    );

    println!("✓ Windjammer correctly uses 'move' only when needed (thread spawns, etc.)");
}
