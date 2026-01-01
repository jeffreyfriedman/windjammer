//! TDD Test: Auto-ref for method arguments
//! WINDJAMMER PHILOSOPHY: Compiler automatically adds & when method signature expects a reference
//!
//! Rules:
//! 1. Method expects &T and we pass T -> add &
//! 2. Method expects T (by value, Copy type) -> do NOT add &
//! 3. Works for stdlib methods (HashMap::remove, String::contains, etc.)
//! 4. Works for custom methods with proper signature lookup

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
fn test_hashmap_remove_adds_ref() {
    // TDD: HashMap::remove(&K) should auto-add & to owned key
    let code = r#"
    use std::collections::HashMap
    
    pub fn remove_item(mut map: HashMap<string, int>, key: string) -> Option<int> {
        return map.remove(key)
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // HashMap::remove expects &K, should add &
    assert!(
        generated.contains("map.remove(&key)"),
        "Should auto-add & for HashMap::remove. Generated:\n{}",
        generated
    );
}

#[test]
fn test_hashmap_get_adds_ref() {
    // TDD: HashMap::get(&K) should auto-add & to owned key
    let code = r#"
    use std::collections::HashMap
    
    pub fn get_value(map: HashMap<string, int>, key: string) -> Option<int> {
        return map.get(key).cloned()
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // HashMap::get expects &K, should add &
    assert!(
        generated.contains("map.get(&key)"),
        "Should auto-add & for HashMap::get. Generated:\n{}",
        generated
    );
}

#[test]
fn test_string_contains_adds_ref() {
    // TDD: String::contains(&str) should auto-add & to owned String
    let code = r#"
    pub fn has_substring(text: string, search: string) -> bool {
        return text.contains(search)
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // String::contains expects &str, should add &
    assert!(
        generated.contains("text.contains(&search)")
            || generated.contains("text.contains(search.as_str())"),
        "Should auto-add & or .as_str() for String::contains. Generated:\n{}",
        generated
    );
}

#[test]
fn test_vec_remove_no_ref() {
    // TDD: Vec::remove(usize) should NOT add & (Copy type passed by value)
    let code = r#"
    pub fn remove_at(mut items: Vec<int>, index: usize) -> int {
        return items.remove(index)
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Vec::remove expects usize by value, should NOT add &
    assert!(
        generated.contains("items.remove(index)") && !generated.contains("items.remove(&index)"),
        "Should NOT add & for Vec::remove (Copy type). Generated:\n{}",
        generated
    );
}

#[test]
fn test_vec_contains_adds_ref() {
    // TDD: Vec::contains(&T) should auto-add & to owned value
    let code = r#"
    pub fn has_item(items: Vec<string>, search: string) -> bool {
        return items.contains(search)
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Vec::contains expects &T, should add &
    assert!(
        generated.contains("items.contains(&search)"),
        "Should auto-add & for Vec::contains. Generated:\n{}",
        generated
    );
}

#[test]
fn test_string_literal_no_ref() {
    // TDD: String literals are already &str, no & needed
    let code = r#"
    pub fn check_text(text: string) -> bool {
        return text.contains("hello")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // String literal is already &str, should NOT add another &
    assert!(
        generated.contains("text.contains(\"hello\")")
            && !generated.contains("text.contains(&\"hello\")"),
        "Should NOT add & to string literals. Generated:\n{}",
        generated
    );
}

#[test]
fn test_mixed_owned_and_literal() {
    // TDD: Mix of owned and literal arguments
    let code = r#"
    use std::collections::HashMap
    
    pub fn test(mut map: HashMap<string, int>, key: string) -> bool {
        map.insert(key, 42);
        return map.contains_key("test")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // insert takes owned key, no & needed (already implemented)
    assert!(
        generated.contains("map.insert(key, 42)") && !generated.contains("map.insert(&key"),
        "insert should not add & to owned key. Generated:\n{}",
        generated
    );

    // contains_key takes &K, but literal is already &str
    assert!(
        generated.contains("map.contains_key(\"test\")")
            && !generated.contains("contains_key(&\"test\")"),
        "contains_key should not add & to literals. Generated:\n{}",
        generated
    );
}

#[test]
fn test_custom_method_with_ref_param() {
    // TDD: Custom methods with & parameters should get auto-ref
    let code = r#"
    pub struct Validator {
    }
    
    impl Validator {
        pub fn check(&self, pattern: &str) -> bool {
            return pattern.len() > 0
        }
    }
    
    pub fn test(validator: Validator, text: string) -> bool {
        return validator.check(text)
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Custom method expects &str, should add & to String
    assert!(
        generated.contains("validator.check(&text)")
            || generated.contains("validator.check(text.as_str())"),
        "Should auto-add & or .as_str() for custom methods. Generated:\n{}",
        generated
    );
}
