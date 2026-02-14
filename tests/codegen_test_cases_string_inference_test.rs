/// TDD Test: String literal to String auto-conversion in @test_cases
///
/// The Windjammer Way: "Compiler Does the Hard Work, Not the Developer"
///
/// Developers should NOT have to write:
///     ["node1".to_string(), "Hello".to_string()]
///
/// They should be able to write:
///     ["node1", "Hello"]
///
/// The compiler should automatically infer and convert string literals to String
/// when the parameter type expects String.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_test_cases_with_string_literals_auto_converts_to_string() {
    let source = r#"
@test_cases([
    ["alice", "Alice", 25],
    ["bob", "Bob", 30]
])
fn test_user(id: string, name: string, age: i32) {
    // String methods should work (not &str)
    assert_eq(id.len() as i32, 5)
    assert_eq(name.len() as i32 > 0, true)
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test_cases_string_inference.wj");
    let output_path = temp_dir
        .path()
        .join("build")
        .join("test_cases_string_inference.rs");

    fs::write(&input_path, source).unwrap();

    // Compile with wj
    let output = Command::new(get_wj_compiler())
        .args(["build", input_path.to_str().unwrap(), "--no-cargo"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Windjammer compilation failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }

    // Read generated Rust code
    let rust_code = fs::read_to_string(&output_path).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // Check that test_user_impl receives String arguments
    // The impl function should take String parameters
    assert!(
        rust_code.contains("fn test_user_impl(id: String, name: String, _age: i32)"),
        "Should generate impl function with String parameters (unused params get _ prefix)"
    );

    // CRITICAL: The test case calls MUST convert &str to String
    // This is the bug we're fixing!
    assert!(rust_code.contains(r#"test_user_impl("alice".to_string(), "Alice".to_string(), 25)"#), 
        "BUG: Compiler must add .to_string() when calling impl function with string literal arguments");

    // Verify it compiles with rustc
    let rustc_result = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if !rustc_result.status.success() {
        panic!(
            "Rustc compilation failed:\n{}",
            String::from_utf8_lossy(&rustc_result.stderr)
        );
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_function_call_with_string_literal_auto_converts() {
    let source = r#"
fn greet(name: string) -> string {
    format!("Hello, {}", name)
}

fn test_greet() {
    let result = greet("World")
    assert_eq(result, "Hello, World".to_string())
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("string_literal_param.wj");
    let output_path = temp_dir
        .path()
        .join("build")
        .join("string_literal_param.rs");

    fs::write(&input_path, source).unwrap();

    // Compile with wj
    let output = Command::new(get_wj_compiler())
        .args(["build", input_path.to_str().unwrap(), "--no-cargo"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Windjammer compilation failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }

    // Read generated Rust code
    let rust_code = fs::read_to_string(&output_path).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // Should contain .to_string() for the parameter
    assert!(
        rust_code.contains(r#"greet("World".to_string())"#),
        "Compiler should auto-convert string literal parameters"
    );

    // Verify it compiles with rustc
    let rustc_result = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if !rustc_result.status.success() {
        panic!(
            "Rustc compilation failed:\n{}",
            String::from_utf8_lossy(&rustc_result.stderr)
        );
    }
}
