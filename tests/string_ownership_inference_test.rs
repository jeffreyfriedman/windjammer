// TDD Test: String ownership inference (&str vs String)
// WINDJAMMER PHILOSOPHY: User writes 'string', compiler infers ownership

use std::process::Command;
use std::fs;

fn compile_code(code: &str) -> Result<String, String> {
    let test_dir = "tests/generated/string_inference_test";
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
    
    assert!(
        generated.contains("text: &str"),
        "Should infer &str for read-only parameter, got:\n{}",
        generated
    );
    
    assert!(
        !generated.contains("\"hello\".to_string()"),
        "Should NOT convert literal for &str param, got:\n{}",
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
