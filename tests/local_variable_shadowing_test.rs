// Test for local variable shadowing of field names
// Compiler bug fix: Local variables should shadow struct fields

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_local_variable_shadows_field() {
    let code = r#"
    pub struct QueryBuilder {
        pub required: Vec<string>,
    }

    impl QueryBuilder {
        pub fn with(self, component_name: string) -> QueryBuilder {
            let mut required = self.required  // Move field into local var
            required.push(component_name)     // Use local var (not self.required!)
            return QueryBuilder {
                required: required,
            }
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should generate: required.push(component_name)
    assert!(generated.contains("required.push(component_name)"), 
        "Local variable 'required' should shadow field, generated:\n{}", generated);
    
    // Should NOT generate: self.required.push(component_name)
    assert!(!generated.contains("self.required.push(component_name)"), 
        "Should not use self.required when local variable shadows it, generated:\n{}", generated);
}

#[test]
fn test_nested_shadowing() {
    let code = r#"
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }

    impl Point {
        pub fn update(self) -> Point {
            let x = self.x + 1  // Shadow x field
            let y = self.y + 1  // Shadow y field
            return Point { x: x, y: y }
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should use local variables x and y, not self.x and self.y
    assert!(generated.contains("Point { x, y }") || generated.contains("Point { x: x, y: y }"), 
        "Should use local variables, generated:\n{}", generated);
}

#[test]
fn test_parameter_does_not_shadow() {
    let code = r#"
    pub struct Counter {
        pub count: i32,
    }

    impl Counter {
        pub fn set(&mut self, count: i32) {
            self.count = count  // Parameter shadows field - use self.count for assignment
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should still generate self.count for field access
    assert!(generated.contains("self.count = count"), 
        "Parameters shadow fields but assignment target should use self.count, generated:\n{}", generated);
}

#[test]
fn test_local_var_method_call() {
    let code = r#"
    pub struct Container {
        pub items: Vec<string>,
    }

    impl Container {
        pub fn add(self, item: string) -> Container {
            let mut items = self.items
            items.push(item)
            return Container { items: items }
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should use local variable items
    assert!(generated.contains("items.push(item)"), 
        "Should use local variable for method call, generated:\n{}", generated);
    assert!(!generated.contains("self.items.push(item)"), 
        "Should not use self.items when shadowed, generated:\n{}", generated);
}

// Helper function to compile Windjammer code and return generated Rust
fn compile_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let input_file = temp_dir.path().join("test.wj");
    let output_dir = temp_dir.path().join("output");

    fs::write(&input_file, code).map_err(|e| format!("Failed to write input file: {}", e))?;

    fs::create_dir(&output_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;

    // Find the wj binary (should be in target/release/wj)
    let mut wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    wj_path.push("target");
    wj_path.push("release");
    wj_path.push("wj");

    let output = Command::new(&wj_path)
        .arg("build")
        .arg(&input_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .output()
        .map_err(|e| format!("Failed to run wj compiler: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Read generated Rust file
    let rust_file = output_dir.join("test.rs");
    fs::read_to_string(&rust_file).map_err(|e| format!("Failed to read generated file: {}", e))
}

