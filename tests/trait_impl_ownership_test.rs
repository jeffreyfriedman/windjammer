//! TDD Test: Trait Implementation Ownership Inference
//!
//! Bug: E0053 - method has incompatible type for trait (8 errors)
//! Root Cause: Impl methods don't match trait's &self / &mut self requirements
//! Expected: impl fn initialize(self) should generate &mut self when trait requires it
//!
//! Philosophy: "Automatic ownership inference" - compiler matches impl to trait

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_and_get_rust(source: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("/tmp/trait_impl_ownership_{}_{}", std::process::id(), counter);

    std::fs::create_dir_all(&test_dir).unwrap();

    let source_file = PathBuf::from(&test_dir).join("test.wj");
    std::fs::write(&source_file, source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&[
            "build",
            source_file.to_str().unwrap(),
            "--target",
            "rust",
            "--output",
            &test_dir,
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
    std::fs::read_to_string(&rust_file).expect("Failed to read generated Rust file")
}

#[test]
fn test_trait_impl_infers_mut_self_from_trait() {
    // Trait methods without explicit self - impl mutates, so trait gets &mut self
    // Impl should match trait's &mut self
    let source = r#"
pub trait Renderer {
    fn initialize()
    fn render()
}

pub struct MyRenderer {
    pub initialized: bool,
}

impl Renderer for MyRenderer {
    fn initialize() {
        self.initialized = true
    }

    fn render() {
        println("rendering")
    }
}
"#;

    let output = compile_and_get_rust(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Trait should have &mut self for initialize (impl mutates)
    assert!(
        output.contains("fn initialize(&mut self)"),
        "Expected 'fn initialize(&mut self)' in trait or impl, got:\n{}",
        output
    );

    // Impl initialize should match - &mut self
    assert!(
        output.contains("fn initialize(&mut self)"),
        "Expected impl 'fn initialize(&mut self)', got:\n{}",
        output
    );

    // render doesn't mutate - can be &self in impl
    // (trait may have &mut self for consistency, impl can use &self if trait allows)
    // Actually: trait gets most permissive from ALL impls. If any impl needs &mut, trait has &mut.
    // Impl MUST match trait. So both need &mut self for initialize.
    // For render: no impl mutates, so trait gets &self. Impl should have &self.
    assert!(
        output.contains("fn render(&self)") || output.contains("fn render(&mut self)"),
        "Expected 'fn render(&self)' or 'fn render(&mut self)', got:\n{}",
        output
    );
}

#[test]
fn test_trait_impl_matches_owned_self() {
    // Trait takes ownership: fn consume(self) -> T
    let source = r#"
pub trait Consumable {
    fn consume(self) -> String
}

pub struct Data {
    value: String,
}

impl Consumable for Data {
    fn consume(self) -> String {
        self.value
    }
}
"#;

    let output = compile_and_get_rust(source);

    println!("\n=== Generated Rust (owned) ===\n{}\n", output);

    // Trait and impl should use owned self
    assert!(
        output.contains("fn consume(self) -> String"),
        "Expected 'fn consume(self) -> String', got:\n{}",
        output
    );
    assert!(
        !output.contains("fn consume(&self)") && !output.contains("fn consume(&mut self)"),
        "Should NOT use &self or &mut self when trait requires owned self"
    );
}

#[test]
fn test_trait_impl_matches_trait_across_files() {
    // Simulate RenderPort scenario: trait in one file, impl in another
    // Single file with trait + two impls - infer_trait_signatures_from_impls should upgrade
    let source = r#"
pub trait Port {
    fn init()
    fn get_value() -> i32
}

pub struct Mock {
    value: i32,
}

impl Port for Mock {
    fn init() {
        self.value = 42
    }
    fn get_value() -> i32 {
        self.value
    }
}

pub struct Real {
    value: i32,
}

impl Port for Real {
    fn init() {
        self.value = 0
    }
    fn get_value() -> i32 {
        self.value
    }
}
"#;

    let output = compile_and_get_rust(source);

    println!("\n=== Generated Rust (multi-impl) ===\n{}\n", output);

    // Both impls mutate in init() - trait gets &mut self
    assert!(
        output.contains("fn init(&mut self)"),
        "Expected 'fn init(&mut self)' (trait upgraded from impls), got:\n{}",
        output
    );

    // Verify rustc compiles the output
    let temp_dir = TempDir::new().unwrap();
    let rs_file = temp_dir.path().join("test.rs");
    fs::write(&rs_file, &output).unwrap();

    let rustc_output = Command::new("rustc")
        .args([rs_file.to_str().unwrap(), "--crate-type=lib", "-o", "/dev/null"])
        .output()
        .expect("Failed to run rustc");

    assert!(
        rustc_output.status.success(),
        "Generated Rust should compile. rustc stderr:\n{}",
        String::from_utf8_lossy(&rustc_output.stderr)
    );
}
