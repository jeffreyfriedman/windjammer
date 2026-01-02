// TDD Test: String ownership inference (&str vs String)
// THE WINDJAMMER WAY: Explicit types are honored
// - User writes `text: string` → `text: String` (owned, as written)
// - User writes `text: &string` → `text: &str` (borrowed, as written)

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
fn test_read_only_param_infers_str_ref() {
    let code = r#"
    pub fn print_msg(text: string) {
        println(text)
    }
    
    pub fn run() {
        print_msg("hello")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // THE WINDJAMMER WAY: Explicit `string` type is honored as `String` (owned)
    // User wrote `text: string` → they want `text: String`, not `text: &str`
    // This prevents API contract violations and maintains consistency
    assert!(
        generated.contains("text: String"),
        "Explicit string type should be honored as String (owned), got:\n{}",
        generated
    );

    assert!(
        generated.contains("\"hello\".to_string()"),
        "Should convert literal to String for String param, got:\n{}",
        generated
    );
}

#[test]
fn test_stored_param_infers_owned() {
    let code = r#"
    pub struct User {
        pub name: string,
    }
    
    impl User {
        pub fn new(name: string) -> User {
            User { name: name }
        }
    }
    
    pub fn run() -> User {
        return User::new("Alice")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("name: String"),
        "Should infer String for stored parameter, got:\n{}",
        generated
    );

    assert!(
        generated.contains("User::new(\"Alice\".to_string())"),
        "SHOULD convert literal for String param, got:\n{}",
        generated
    );
}
