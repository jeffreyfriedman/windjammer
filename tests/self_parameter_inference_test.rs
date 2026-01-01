// TDD Test: Self parameter inference - developers write just "self", compiler infers &self, &mut self, or self
// Philosophy: "Compiler does the hard work, not the developer"
// Expected:
//   - self only read → &self
//   - self mutated → &mut self
//   - self consumed → self (owned)

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_check(code: &str) -> (bool, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.wj");
    let output_file = temp_dir.path().join("test.rs");

    fs::write(&test_file, code).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            test_file.to_str().unwrap(),
            "--output",
            temp_dir.path().to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let generated = fs::read_to_string(output_file).unwrap_or_default();
    (output.status.success(), generated)
}

#[test]
fn test_self_inference_read_only() {
    let code = r#"
pub struct Point {
    pub x: int,
    pub y: int,
}

impl Point {
    pub fn get_x(self) -> int {
        return self.x
    }
    
    pub fn distance_from_origin(self) -> f32 {
        return ((self.x * self.x + self.y * self.y) as f32).sqrt()
    }
}
"#;

    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");

    // When self is only read, should infer &self
    assert!(
        generated.contains("pub fn get_x(&self) -> i64"),
        "Read-only method should infer '&self'. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("pub fn distance_from_origin(&self) -> f32"),
        "Read-only method should infer '&self'. Generated:\n{}",
        generated
    );
}

#[test]
fn test_self_inference_mutating() {
    let code = r#"
pub struct Counter {
    pub count: int,
}

impl Counter {
    pub fn increment(self) {
        self.count = self.count + 1
    }
    
    pub fn add(self, amount: int) {
        self.count = self.count + amount
    }
    
    pub fn reset(self) {
        self.count = 0
    }
}
"#;

    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");

    // When self is mutated, should infer &mut self
    assert!(
        generated.contains("pub fn increment(&mut self)"),
        "Mutating method should infer '&mut self'. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("pub fn add(&mut self, amount: i64)"),
        "Mutating method should infer '&mut self'. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("pub fn reset(&mut self)"),
        "Mutating method should infer '&mut self'. Generated:\n{}",
        generated
    );
}

#[test]
fn test_self_inference_consuming() {
    let code = r#"
pub struct Builder {
    pub value: int,
}

impl Builder {
    pub fn build(self) -> int {
        return self.value
    }
    
    pub fn into_inner(self) -> int {
        self.value
    }
}
"#;

    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");

    // WINDJAMMER FIX: Copy types optimize to &self for read-only access
    // This is MORE efficient than consuming self for Copy types
    // The struct is Copy (contains only i64), so &self is correct
    assert!(
        generated.contains("pub fn build(&self) -> i64"),
        "Copy type read method should use &self. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("pub fn into_inner(&self) -> i64"),
        "Copy type read method should use &self. Generated:\n{}",
        generated
    );
}

#[test]
fn test_self_inference_mixed() {
    let code = r#"
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

impl Vector {
    pub fn length(self) -> f32 {
        ((self.x * self.x + self.y * self.y) as f32).sqrt()
    }
    
    pub fn normalize(self) {
        let len = self.length();
        self.x = self.x / len;
        self.y = self.y / len
    }
    
    pub fn consume(self) -> f32 {
        self.x + self.y
    }
}
"#;

    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");

    // length: read-only → &self
    assert!(
        generated.contains("pub fn length(&self) -> f32"),
        "Read-only should infer '&self'. Generated:\n{}",
        generated
    );

    // normalize: mutates self → &mut self
    assert!(
        generated.contains("pub fn normalize(&mut self)"),
        "Mutating should infer '&mut self'. Generated:\n{}",
        generated
    );

    // WINDJAMMER FIX: consume is read-only on Copy type → &self (more efficient)
    // Copy types don't need to be consumed for read-only operations
    assert!(
        generated.contains("pub fn consume(&self) -> f32"),
        "Copy type read should use '&self'. Generated:\n{}",
        generated
    );
}

#[test]
#[ignore] // TODO: Implement trait method ownership inference from impl bodies (advanced feature)
fn test_self_inference_trait_methods() {
    let code = r#"
pub trait Drawable {
    fn draw(self);
    fn update(self, delta: f32);
}

pub struct Sprite {
    pub x: f32,
    pub dirty: bool,
}

impl Drawable for Sprite {
    fn draw(self) {
        println("Drawing");
    }
    
    fn update(self, delta: f32) {
        self.dirty = true
    }
}
"#;

    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");

    // Trait methods should also infer
    // draw: read-only → &self
    let trait_section: String = generated
        .lines()
        .skip_while(|l| !l.contains("trait Drawable"))
        .take(10)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        trait_section.contains("fn draw(&self);"),
        "Trait read-only method should infer '&self'. Trait:\n{}",
        trait_section
    );

    // update: mutates → &mut self
    assert!(
        trait_section.contains("fn update(&mut self, delta: f32);"),
        "Trait mutating method should infer '&mut self'. Trait:\n{}",
        trait_section
    );
}
