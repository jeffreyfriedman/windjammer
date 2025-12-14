// TDD Test: Compiler should NOT add .to_string() when function expects &str
// WINDJAMMER PHILOSOPHY: Smart inference - only convert when needed

use std::process::Command;
use std::fs;

fn compile_code(code: &str) -> Result<String, String> {
    let test_dir = "tests/generated/string_literal_test";
    fs::create_dir_all(test_dir).expect("Failed to create test dir");
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
            test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        fs::remove_dir_all(test_dir).ok();
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_file = format!("{}/test.rs", test_dir);
    let generated = fs::read_to_string(&generated_file)
        .expect("Failed to read generated file");

    fs::remove_dir_all(test_dir).ok();

    Ok(generated)
}

#[test]
fn test_string_literal_to_str_param_no_conversion() {
    // BUG: Compiler incorrectly adds .to_string() when function expects &str
    let code = r#"
    pub fn process(text: &str) -> int {
        return text.len() as int
    }
    
    pub fn run() -> int {
        return process("hello")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should NOT add .to_string() for &str parameter
    assert!(
        !generated.contains("\"hello\".to_string()"),
        "Should NOT add .to_string() when function expects &str, got:\n{}",
        generated
    );
    
    // Should pass string literal directly
    assert!(
        generated.contains("process(\"hello\")"),
        "Should pass string literal directly, got:\n{}",
        generated
    );
}

#[test]
fn test_ffi_function_str_param() {
    // Real case: FFI functions expecting &str
    let code = r#"
    extern fn load_sound(path: &str) -> int
    
    pub fn init() -> int {
        return load_sound("assets/sound.ogg")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should NOT convert string literals for FFI &str params
    assert!(
        !generated.contains(".to_string()"),
        "Should NOT add .to_string() for FFI &str parameter, got:\n{}",
        generated
    );
}

#[test]
fn test_method_call_with_str_param() {
    let code = r#"
    pub struct Loader;
    
    impl Loader {
        pub fn load(&self, path: &str) -> int {
            return path.len() as int
        }
    }
    
    pub fn run() -> int {
        let loader = Loader;
        return loader.load("data.txt")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Should NOT add .to_string() for method with &str param
    assert!(
        !generated.contains("\"data.txt\".to_string()"),
        "Should NOT add .to_string() for method with &str param, got:\n{}",
        generated
    );
}

#[test]
fn test_string_param_needs_conversion() {
    // When function expects String (owned), DO convert
    let code = r#"
    pub fn process(text: string) -> int {
        return text.len() as int
    }
    
    pub fn run() -> int {
        return process("hello")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // SHOULD add .to_string() for String parameter
    assert!(
        generated.contains("\"hello\".to_string()"),
        "SHOULD add .to_string() when function expects String, got:\n{}",
        generated
    );
}

#[test]
fn test_mixed_str_and_string_params() {
    let code = r#"
    pub fn process_str(text: &str) -> int {
        return text.len() as int
    }
    
    pub fn process_string(text: string) -> int {
        return text.len() as int
    }
    
    pub fn run() {
        process_str("no conversion")
        process_string("yes conversion")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");
    
    // Check both cases
    assert!(
        generated.contains("process_str(\"no conversion\")"),
        "Should NOT convert for &str param, got:\n{}",
        generated
    );
    assert!(
        generated.contains("process_string(\"yes conversion\".to_string())"),
        "SHOULD convert for String param, got:\n{}",
        generated
    );
}

