use std::env;
/// TDD Test: @test_cases Decorator Parsing
///
/// THE WINDJAMMER WAY: Test decorators with array syntax should parse correctly
///
/// This test reproduces the "Expected pattern, got Assign" parser error
/// that occurs when parsing @test_cases decorator with array arguments
use std::fs;
use std::path::PathBuf;

#[test]
fn test_test_cases_decorator_parses() {
    // Create a minimal test file with @test_cases decorator
    let test_dir = std::env::temp_dir().join(format!(
        "wj_test_cases_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Test file with @test_cases decorator
    let test_content = r#"
use windjammer_runtime::test::*

@test_cases([
    [5, 5],
    [10, 10]
])
fn test_capacity(capacity: i32, expected: i32) {
    assert_eq!(capacity, expected, "Should match")
}
"#;

    fs::write(test_dir.join("test_cases.wj"), test_content).unwrap();

    // Try to parse the file
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("test_cases.wj"))
        .output()
        .expect("Failed to run wj build");

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Assert no "Expected pattern, got Assign" error
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stderr.contains("Expected pattern, got Assign")
        || stdout.contains("Expected pattern, got Assign")
    {
        panic!(
            "Parser error 'Expected pattern, got Assign' occurred:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }

    // Should compile successfully (or at least not have parser errors)
    // Note: May fail with other errors (missing dependencies, etc.) but not parser errors
}

#[test]
fn test_simple_test_decorator_works() {
    // Create a minimal test file with @test decorator (should work)
    let test_dir = std::env::temp_dir().join(format!(
        "wj_simple_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Simple test file with @test decorator
    let test_content = r#"
use windjammer_runtime::test::*

@test
fn test_basic() {
    let x = 1 + 1
    assert_eq!(x, 2, "Math works")
}
"#;

    fs::write(test_dir.join("simple_test.wj"), test_content).unwrap();

    // Try to parse the file
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("simple_test.wj"))
        .output()
        .expect("Failed to run wj build");

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Assert no parser errors
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !stderr.contains("Expected pattern") && !stdout.contains("Expected pattern"),
        "Should not have parser errors for simple @test decorator:\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}
