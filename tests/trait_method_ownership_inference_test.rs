/// TDD Test: Trait Method Ownership Inference
///
/// Bug: Trait methods without explicit &mut self don't infer ownership
/// Root Cause: Analyzer doesn't infer self parameter for trait methods
/// Expected: fn initialize() → fn initialize(&mut self)
///          fn get_name() → fn get_name(&self)

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_trait_method_infers_mut_self() {
    let source = r#"
pub trait Counter {
    fn increment()
    fn reset()
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Trait methods should have &mut self inferred
    assert!(
        output.contains("fn increment(&mut self)"),
        "Expected 'fn increment(&mut self)', got: {}",
        output
    );
    assert!(
        output.contains("fn reset(&mut self)"),
        "Expected 'fn reset(&mut self)', got: {}",
        output
    );
}

#[test]
fn test_trait_method_infers_mut_self_by_default() {
    // THE WINDJAMMER WAY: Trait methods without bodies default to &mut self
    // This is the most permissive signature (allows all implementations)
    // Individual implementations can use &self if they don't need mutation
    let source = r#"
pub trait Readable {
    fn get_value() -> i32
    fn is_empty() -> bool
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Trait methods default to &mut self (most permissive)
    assert!(
        output.contains("fn get_value(&mut self) -> i32"),
        "Expected 'fn get_value(&mut self) -> i32' (default for traits), got: {}",
        output
    );
    assert!(
        output.contains("fn is_empty(&mut self) -> bool"),
        "Expected 'fn is_empty(&mut self) -> bool' (default for traits), got: {}",
        output
    );
}

#[test]
fn test_trait_method_with_params_infers_mut_self() {
    let source = r#"
pub trait Renderer {
    fn set_camera(camera: i32)
    fn upload_data(data: Vec<u8>)
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Methods with parameters should infer &mut self
    assert!(
        output.contains("fn set_camera(&mut self, camera: i32)"),
        "Expected 'fn set_camera(&mut self, camera: i32)', got: {}",
        output
    );
    assert!(
        output.contains("fn upload_data(&mut self, data: Vec<u8>)"),
        "Expected 'fn upload_data(&mut self, data: Vec<u8>)', got: {}",
        output
    );
}

#[test]
fn test_trait_impl_infers_self_from_trait() {
    let source = r#"
pub trait Incrementable {
    fn increment()
}

pub struct Counter {
    count: i32,
}

impl Incrementable for Counter {
    fn increment() {
        self.count = self.count + 1
    }
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Trait definition should infer &mut self
    assert!(
        output.contains("fn increment(&mut self)"),
        "Expected trait method with &mut self, got: {}",
        output
    );
    
    // Impl should match trait signature
    // The impl should also have &mut self (from trait)
}

#[test]
fn test_associated_functions_no_self() {
    let source = r#"
pub trait Factory {
    fn new() -> Factory
    fn default() -> Factory
}
"#;

    let output = compile_and_get_rust(source);
    
    println!("\n=== Generated Rust ===\n{}\n", output);
    
    // Associated functions (constructors) should NOT have self
    assert!(
        output.contains("fn new() -> ") && !output.contains("fn new(&"),
        "Expected 'fn new()' without self (associated function), got: {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
fn compile_and_get_rust(source: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/trait_ownership_test_{}_{}", std::process::id(), counter);
    
    std::fs::create_dir_all(&test_dir).unwrap();
    
    let source_file = PathBuf::from(&test_dir).join("test.wj");
    std::fs::write(&source_file, source).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            source_file.to_str().unwrap(),
            "--target", "rust",
            "--output", &test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");
    
    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    let rust_file = PathBuf::from(&test_dir).join("test.rs");
    std::fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file")
}
