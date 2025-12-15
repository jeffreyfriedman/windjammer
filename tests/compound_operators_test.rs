// TDD Test: Compound operators should be preserved in generated code
// Bug: `x += 1` expands to `x = x + 1` instead of `x += 1`
// Expected: Preserve compound operators for cleaner, more idiomatic Rust

use std::fs;
use std::process::Command;

fn compile_and_check(code: &str) -> (bool, String) {
    let test_file = "/tmp/compound_test.wj";
    let output_file = "/tmp/compound_test.rs";
    
    fs::write(test_file, code).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(&["build", test_file, "--output", "/tmp", "--no-cargo"])
        .output()
        .expect("Failed to run wj");
    
    let generated = fs::read_to_string(output_file).unwrap_or_default();
    (output.status.success(), generated)
}

#[test]
fn test_compound_addition() {
    let code = r#"
pub struct Counter {
    pub count: int,
}

impl Counter {
    pub fn increment(self) {
        self.count += 1
    }
}
"#;
    
    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");
    
    // Should preserve += operator
    assert!(generated.contains("self.count += 1"), 
        "Should preserve '+=' operator, not expand it. Generated:\n{}", generated);
    assert!(!generated.contains("self.count = self.count + 1"), 
        "Should NOT expand to 'x = x + 1'. Generated:\n{}", generated);
}

#[test]
fn test_compound_all_operators() {
    let code = r#"
pub struct Math {
    pub value: int,
}

impl Math {
    pub fn ops(self, x: int) {
        self.value += x;
        self.value -= x;
        self.value *= x;
        self.value /= x
    }
}
"#;
    
    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");
    
    // Should preserve all compound operators
    assert!(generated.contains("self.value += x"), 
        "Should preserve '+=' operator. Generated:\n{}", generated);
    assert!(generated.contains("self.value -= x"), 
        "Should preserve '-=' operator. Generated:\n{}", generated);
    assert!(generated.contains("self.value *= x"), 
        "Should preserve '*=' operator. Generated:\n{}", generated);
    assert!(generated.contains("self.value /= x"), 
        "Should preserve '/=' operator. Generated:\n{}", generated);
}

#[test]
fn test_compound_with_field_access() {
    let code = r#"
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn add(self, other: Vec2) {
        self.x += other.x;
        self.y += other.y
    }
}
"#;
    
    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");
    
    assert!(generated.contains("self.x += other.x"), 
        "Should preserve compound operator with field access. Generated:\n{}", generated);
    assert!(generated.contains("self.y += other.y"), 
        "Should preserve compound operator with field access. Generated:\n{}", generated);
}


