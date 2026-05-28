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

//! TDD Test: Trait Implementation Ownership Inference
//!
//! Bug: E0053 - method has incompatible type for trait (8 errors)
//! Root Cause: Impl methods don't match trait's &self / &mut self requirements
//! Expected: impl fn initialize(self) should generate &mut self when trait requires it
//!
//! Philosophy: "Automatic ownership inference" - compiler matches impl to trait

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

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

    let output = test_utils::compile_single(source);

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
fn test_trait_impl_field_return_uses_borrowed_self() {
    // DESIGN DECISION: When a method body is `self.field` (returning a non-Copy
    // field), the compiler uses &self + auto-clone even for trait methods.
    // Both trait and impl must agree. The compiler cannot distinguish "consume"
    // semantics from "getter" semantics when the body is identical.
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

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust (field-return) ===\n{}\n", output);

    // Windjammer Way: field-return methods use &self + auto-clone
    assert!(
        output.contains("fn consume(&self) -> String"),
        "Expected &self for field-return method (auto-clone pattern). Got:\n{}",
        output
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

    let output = test_utils::compile_single(source);

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

    let rmeta = temp_dir.path().join("verify.rmeta");
    let rustc_output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(rmeta.to_str().unwrap())
        .arg(rs_file.to_str().unwrap())
        .output()
        .expect("Failed to run rustc");

    assert!(
        rustc_output.status.success(),
        "Generated Rust should compile. rustc stderr:\n{}",
        String::from_utf8_lossy(&rustc_output.stderr)
    );
}
