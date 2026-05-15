// Integration test for module declarations
// Verifies that module declarations parse and generate correct Rust code

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_module_declarations() {
    let source = r#"
// Simple module declaration
mod utils;

// Public module declaration
pub mod math;
pub mod physics;

// Multiple modules
pub mod rendering;
pub mod audio;
pub mod world;

// Private module
mod internal;
mod helpers;
"#;

    let (rust_code, success) = test_utils::compile_single_check(source);

    assert!(success, "Module declarations should parse successfully");

    // Verify generated Rust contains module declarations (as inline modules)
    // Note: Current implementation generates inline modules `mod x { }` not external `mod x;`
    assert!(
        rust_code.contains("mod utils"),
        "Should generate 'mod utils'"
    );
    assert!(
        rust_code.contains("pub mod math"),
        "Should generate 'pub mod math'"
    );
    assert!(
        rust_code.contains("pub mod physics"),
        "Should generate 'pub mod physics'"
    );
    assert!(
        rust_code.contains("pub mod rendering"),
        "Should generate 'pub mod rendering'"
    );
    assert!(
        rust_code.contains("pub mod audio"),
        "Should generate 'pub mod audio'"
    );
    assert!(
        rust_code.contains("pub mod world"),
        "Should generate 'pub mod world'"
    );
    assert!(
        rust_code.contains("mod internal"),
        "Should generate 'mod internal'"
    );
    assert!(
        rust_code.contains("mod helpers"),
        "Should generate 'mod helpers'"
    );

    println!("✓ Module declarations parse and generate correctly");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_inline_module() {
    let source = r#"
pub mod utils {
    pub fn helper() -> i32 {
        42
    }
}
"#;

    let (rust_code, success) = test_utils::compile_single_check(source);

    assert!(success, "Inline modules should parse successfully");

    // Verify generated Rust contains inline module
    assert!(
        rust_code.contains("pub mod utils"),
        "Should generate 'pub mod utils'"
    );
    assert!(
        rust_code.contains("pub fn helper() -> i32"),
        "Should contain function inside module"
    );
    assert!(rust_code.contains("42"), "Should contain function body");

    println!("✓ Inline modules parse and generate correctly");
}
