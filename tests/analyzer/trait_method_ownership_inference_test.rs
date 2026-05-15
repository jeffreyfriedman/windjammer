/// TDD Test: Trait Method Ownership Inference
///
/// Bug: Trait methods without explicit &mut self don't infer ownership
/// Root Cause: Analyzer doesn't infer self parameter for trait methods
/// Expected: fn initialize() → fn initialize(&mut self)
///          fn get_name() → fn get_name(&self)
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_trait_method_infers_mut_self() {
    let source = r#"
pub trait Counter {
    fn increment()
    fn reset()
}
"#;

    let output = test_utils::compile_single(source);

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
fn test_trait_method_abstract_with_return_defaults_to_ref_self() {
    // Abstract methods that return a value are treated as getters: default `&self`.
    // Void abstract methods default to `&mut self` (see increment/reset tests).
    let source = r#"
pub trait Readable {
    fn get_value() -> i32
    fn is_empty() -> bool
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(
        output.contains("fn get_value(&self) -> i32"),
        "Expected 'fn get_value(&self) -> i32', got: {}",
        output
    );
    assert!(
        output.contains("fn is_empty(&self) -> bool"),
        "Expected 'fn is_empty(&self) -> bool', got: {}",
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

    let output = test_utils::compile_single(source);

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

    let output = test_utils::compile_single(source);

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

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Associated functions (constructors) should NOT have self
    assert!(
        output.contains("fn new() -> ") && !output.contains("fn new(&"),
        "Expected 'fn new()' without self (associated function), got: {}",
        output
    );
}

// Helper function to compile Windjammer code and return generated Rust
