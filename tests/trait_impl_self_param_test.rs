// Test: Trait implementation should match trait method signatures
//
// Bug: Analyzer infers `&self` for methods that access fields, 
// but trait requires `self` (owned). This causes E0053 errors.
//
// Expected: When implementing a trait method, use the trait's 
// self parameter type, not the inferred type.

use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn compile_windjammer_code(code: &str) -> Result<String, String> {
    // Use unique directory for each test to avoid conflicts
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = format!("tests/generated/trait_impl_test_{}", test_id);
    fs::create_dir_all(&test_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test.wj", test_dir);
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            &input_file,
            "--output",
            &test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    let generated_file = format!("{}/test.rs", test_dir);
    let generated = fs::read_to_string(&generated_file)
        .unwrap_or_else(|_| String::from_utf8_lossy(&output.stdout).to_string());

    // Clean up test directory
    fs::remove_dir_all(&test_dir).ok();

    if output.status.success() {
        Ok(generated)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[test]
fn test_trait_impl_self_param_owned() {
    // TDD: This test will FAIL until we fix the analyzer
    
    let code = r#"
        trait Renderable {
            fn render(self) -> string
        }
        
        struct Text {
            content: string
        }
        
        impl Renderable for Text {
            fn render(self) -> string {
                self.content
            }
        }
    "#;
    
    let result = compile_windjammer_code(code);
    
    // Should compile successfully
    assert!(result.is_ok(), "Trait impl should compile: {:?}", result.err());
    
    let generated = result.unwrap();
    
    // Verify generated Rust uses owned self (matches trait)
    assert!(generated.contains("fn render(self) -> String"), 
            "Expected 'fn render(self)' but got:\n{}", generated);
    assert!(!generated.contains("fn render(&self)"), 
            "Should NOT use &self when trait requires self");
}

#[test]
fn test_trait_impl_self_param_borrowed() {
    // TDD: Test that &self in trait is respected
    
    let code = r#"
        trait Displayable {
            fn display(&self) -> string
        }
        
        struct Label {
            text: string
        }
        
        impl Displayable for Label {
            fn display(&self) -> string {
                self.text.clone()
            }
        }
    "#;
    
    let result = compile_windjammer_code(code);
    
    // Should compile successfully
    assert!(result.is_ok(), "Trait impl should compile: {:?}", result.err());
    
    let generated = result.unwrap();
    
    // Verify generated Rust uses &self (matches trait)
    assert!(generated.contains("fn display(&self) -> String"), 
            "Expected 'fn display(&self)' but got:\n{}", generated);
}

#[test]
fn test_trait_impl_self_param_mutable() {
    // TDD: Test that &mut self in trait is respected
    
    let code = r#"
        trait Updatable {
            fn update(&mut self, value: int)
        }
        
        struct Counter {
            count: int
        }
        
        impl Updatable for Counter {
            fn update(&mut self, value: int) {
                self.count = value
            }
        }
    "#;
    
    let result = compile_windjammer_code(code);
    
    // Should compile successfully
    assert!(result.is_ok(), "Trait impl should compile: {:?}", result.err());
    
    let generated = result.unwrap();
    
    // Verify generated Rust uses &mut self (matches trait)
    assert!(generated.contains("fn update(&mut self, value: i64)"), 
            "Expected 'fn update(&mut self, value: i64)' but got:\n{}", generated);
}

