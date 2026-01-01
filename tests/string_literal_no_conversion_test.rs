// TDD Test: Compiler should NOT add .to_string() when function expects &str
// WINDJAMMER PHILOSOPHY: Smart inference - only convert when needed

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
    // WINDJAMMER FIX: When string parameter is only read, it's inferred to &str
    // This is more efficient - no .to_string() needed at call site
    let code = r#"
    pub fn process(text: string) -> int {
        return text.len() as int
    }
    
    pub fn run() -> int {
        return process("hello")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // THE WINDJAMMER WAY: Explicit `string` type is honored as `String` (owned)
    // User wrote `text: string` → they want `text: String`, not `text: &str`
    assert!(
        generated.contains("pub fn process(text: String)"),
        "Explicit string type should be honored as String (owned), got:\n{}",
        generated
    );
    assert!(
        generated.contains("\"hello\".to_string()"),
        "Should add .to_string() when function expects String (owned), got:\n{}",
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

    // THE WINDJAMMER WAY: Explicit types are honored
    // - `text: &str` stays `&str` (no conversion)
    // - `text: string` becomes `String` (owned, with conversion)
    assert!(
        generated.contains("process_str(\"no conversion\")"),
        "Should NOT convert for explicit &str param, got:\n{}",
        generated
    );
    assert!(
        generated.contains("pub fn process_string(text: String)"),
        "Explicit string type should be honored as String (owned), got:\n{}",
        generated
    );
    assert!(
        generated.contains(".to_string()"),
        "Should convert for String param, got:\n{}",
        generated
    );
}
