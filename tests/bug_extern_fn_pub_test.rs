#[path = "test_utils.rs"]
mod test_utils;

/// TDD test: extern fn declarations should generate `pub fn` inside `extern "C"` blocks
///
/// Bug: extern fn declarations generated as private, making them inaccessible
/// from other modules via `pub use module::*;` re-exports.
///
/// Root Cause: generate_extern_function() emitted `fn name(...)` without `pub`.
///
/// Fix: Emit `pub fn name(...)` for extern function declarations.
#[test]
fn test_extern_fn_generates_pub() {
    let source = r#"
extern fn do_something(x: i32, y: i32) -> i32
extern fn do_nothing()
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("extern \"C\""),
        "Should generate extern C block"
    );
    assert!(
        generated.contains("pub fn do_something("),
        "extern fn should generate pub fn, got:\n{}",
        generated
    );
    assert!(
        generated.contains("pub fn do_nothing("),
        "extern fn should generate pub fn, got:\n{}",
        generated
    );
}
