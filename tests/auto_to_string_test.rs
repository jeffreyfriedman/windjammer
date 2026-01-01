//! TDD Test: Auto .to_string() for string literals in method calls
//! WINDJAMMER PHILOSOPHY: Compiler does the work - automatically convert string literals to String when needed
//!
//! Rules:
//! 1. Method expects String -> add .to_string() to string literals
//! 2. Method expects &str -> do NOT add .to_string()
//! 3. Works for Vec::push(), custom methods, any method expecting String

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
fn test_vec_push_string_literal() {
    // TDD: Vec<String>::push("literal") should add .to_string()
    let code = r#"
    pub fn create_list() -> Vec<string> {
        let mut items = Vec::new()
        items.push("first")
        items.push("second")
        items
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should automatically add .to_string() for string literals
    assert!(
        generated.contains("push(\"first\".to_string())"),
        "Should auto-convert first string literal. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("push(\"second\".to_string())"),
        "Should auto-convert second string literal. Generated:\n{}",
        generated
    );
}

#[test]
fn test_vec_push_string_variable_no_conversion() {
    // TDD: Vec<String>::push(variable) should NOT add .to_string()
    let code = r#"
    pub fn add_item(item: string) -> Vec<string> {
        let mut items = Vec::new()
        items.push(item)
        items
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should NOT add .to_string() for variables (already String)
    assert!(
        generated.contains("push(item)") && !generated.contains("push(item.to_string())"),
        "Should NOT convert string variables. Generated:\n{}",
        generated
    );
}

#[test]
fn test_custom_method_string_param() {
    // TDD: Custom method expecting String should auto-convert literals
    let code = r#"
    pub struct Logger {
        messages: Vec<string>
    }
    
    impl Logger {
        pub fn log(&mut self, message: string) {
            self.messages.push(message)
        }
    }
    
    pub fn test() {
        let mut logger = Logger { messages: Vec::new() }
        logger.log("Hello, World!")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should add .to_string() when calling log() with a string literal
    assert!(
        generated.contains("log(\"Hello, World!\".to_string())"),
        "Should auto-convert string literal in custom method. Generated:\n{}",
        generated
    );
}

#[test]
fn test_str_param_no_conversion() {
    // TDD: Methods expecting &str should NOT add .to_string()
    let code = r#"
    pub struct Text {
        content: string
    }
    
    impl Text {
        pub fn starts_with(&self, prefix: &str) -> bool {
            return self.content.starts_with(prefix)
        }
    }
    
    pub fn test(text: Text) -> bool {
        return text.starts_with("Hello")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should NOT add .to_string() for &str parameters
    assert!(
        generated.contains("starts_with(\"Hello\")") && !generated.contains(".to_string()"),
        "Should NOT convert for &str parameters. Generated:\n{}",
        generated
    );
}

#[test]
fn test_multiple_string_params() {
    // TDD: Multiple String parameters should all get .to_string()
    let code = r#"
    pub fn concatenate(a: string, b: string, c: string) -> string {
        return a + &b + &c
    }
    
    pub fn test() -> string {
        return concatenate("Hello", "World", "!")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // All three literals should be converted
    assert!(
        generated.contains(
            "concatenate(\"Hello\".to_string(), \"World\".to_string(), \"!\".to_string())"
        ),
        "Should convert all string literal arguments. Generated:\n{}",
        generated
    );
}

#[test]
fn test_mixed_str_and_string_params() {
    // TDD: Mixed &str and String parameters
    let code = r#"
    pub fn process(name: string, suffix: &str) -> string {
        return name + suffix
    }
    
    pub fn test() -> string {
        return process("test", ".txt")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // First param (String) should convert, second (&str) should not
    assert!(
        generated.contains("process(\"test\".to_string(), \".txt\")"),
        "Should convert String param but not &str param. Generated:\n{}",
        generated
    );
}

#[test]
fn test_chained_method_calls() {
    // TDD: Chained method calls with String parameters
    let code = r#"
    pub struct Builder {
        value: string
    }
    
    impl Builder {
        pub fn new() -> Builder {
            return Builder { value: "".to_string() }
        }
        
        pub fn with_value(mut self, val: string) -> Builder {
            self.value = val
            return self
        }
        
        pub fn build(self) -> string {
            return self.value
        }
    }
    
    pub fn test() -> string {
        return Builder::new().with_value("hello").build()
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should convert the string literal in the chained call
    assert!(
        generated.contains("with_value(\"hello\".to_string())"),
        "Should convert string literal in chained call. Generated:\n{}",
        generated
    );
}

#[test]
fn test_hashmap_insert_string_keys() {
    // TDD: HashMap<String, T>::insert should convert key literals
    let code = r#"
    use std::collections::HashMap
    
    pub fn create_map() -> HashMap<string, int> {
        let mut map = HashMap::new()
        map.insert("key1", 1)
        map.insert("key2", 2)
        return map
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should convert string literal keys for HashMap<String, _>
    assert!(
        generated.contains("insert(\"key1\".to_string(), 1)"),
        "Should convert HashMap string key literals. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("insert(\"key2\".to_string(), 2)"),
        "Should convert HashMap string key literals. Generated:\n{}",
        generated
    );
}
