//! TDD Test: Trait method declarations without bodies
//! WINDJAMMER PHILOSOPHY: Traits should support method declarations without bodies
//! This enables proper trait definitions that implementations must fulfill

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_file = test_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_file).expect("Failed to read generated file");

    Ok(generated)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_no_body_single() {
    // TDD: Simple trait with single method without body
    let code = r#"
    pub trait Drawable {
        fn draw(&self);
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should generate trait with method declaration ending in semicolon
    assert!(
        generated.contains("fn draw(&self);"),
        "Trait method without body should end with semicolon. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_no_body_multiple() {
    // TDD: Trait with multiple methods without bodies
    let code = r#"
    pub trait GameLoop {
        fn init(&mut self);
        fn update(&mut self, delta: f32);
        fn render(&self);
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("fn init(&mut self);"),
        "Expected init method. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn update(&mut self, delta: f32);"),
        "Expected update method. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn render(&self);"),
        "Expected render method. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_mixed_bodies() {
    // TDD: Trait with some methods having default implementations and some not
    let code = r#"
    pub trait Updatable {
        fn update(&mut self);
        
        fn tick(&mut self) {
            self.update()
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Method without body should have semicolon
    assert!(
        generated.contains("fn update(&mut self);"),
        "Method without body should end with semicolon. Generated:\n{}",
        generated
    );

    // Method with default impl should have body
    assert!(
        generated.contains("fn tick(&mut self) {"),
        "Method with default impl should have body. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_with_no_body_trait() {
    // TDD: Full trait + impl scenario
    let code = r#"
    pub trait Drawable {
        fn draw(&self);
        fn update(&mut self, delta: f32);
    }
    
    pub struct Sprite {
        pub x: f32,
        pub y: f32,
    }
    
    impl Drawable for Sprite {
        fn draw(&self) {
            let _pos = self.x + self.y
        }
        
        fn update(&mut self, delta: f32) {
            self.x = self.x + delta;
            self.y = self.y + delta
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Trait should have method declarations
    assert!(
        generated.contains("fn draw(&self);"),
        "Trait method should end with semicolon. Generated:\n{}",
        generated
    );

    // Impl should have method bodies
    assert!(
        generated.contains("fn draw(&self) {") || generated.contains("pub fn draw(&self) {"),
        "Impl method should have body. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_with_return_type() {
    // TDD: Trait method without body but with return type
    let code = r#"
    pub trait Calculator {
        fn add(&self, a: int, b: int) -> int;
        fn multiply(&self, a: int, b: int) -> int;
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("fn add(&self, a: i64, b: i64) -> i64;"),
        "Method with return type should end with semicolon. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn multiply(&self, a: i64, b: i64) -> i64;"),
        "Method with return type should end with semicolon. Generated:\n{}",
        generated
    );
}
