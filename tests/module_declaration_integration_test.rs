// Integration test for module declarations
// Verifies that module declarations parse and generate correct Rust code

use std::fs;
use std::process::Command;

fn compile_wj(source: &str) -> (String, bool) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let process_id = std::process::id();
    let unique_id = format!("{}_{}", process_id, test_id);

    let temp_dir = std::env::temp_dir();
    let test_file = format!("test_mod_{}.wj", unique_id);
    let temp_file = temp_dir.join(&test_file);
    fs::write(&temp_file, source).expect("Failed to write temp file");

    let output_dir = temp_dir.join(format!("output_mod_{}", unique_id));
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--bin",
            "wj",
            "--",
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            temp_file.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !success {
        eprintln!("Compilation failed:");
        eprintln!("STDERR: {}", stderr);
    }

    // Read generated Rust code
    let rust_file = output_dir.join(format!("{}.rs", test_file.replace(".wj", "")));
    let rust_code = if rust_file.exists() {
        fs::read_to_string(&rust_file).unwrap_or_default()
    } else {
        String::new()
    };

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_dir_all(&output_dir);

    (rust_code, success)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
#[ignore] // TODO: Implement module declaration code generation
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

    let (rust_code, success) = compile_wj(source);

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
#[ignore] // TODO: Inline modules require recursive analysis - tracked in TODO_MODULE_DECLARATIONS.md
fn test_inline_module() {
    let source = r#"
pub mod utils {
    pub fn helper() -> i32 {
        42
    }
}
"#;

    let (rust_code, success) = compile_wj(source);

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
