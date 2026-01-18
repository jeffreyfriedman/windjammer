use std::env;
/// TDD Test: Decorator Keyword Arguments Parsing
///
/// THE WINDJAMMER WAY: Decorators with keyword arguments should parse correctly
///
/// This test reproduces the "Expected pattern, got Assign" error with
/// @property_test(iterations=50, seed=42) syntax
use std::fs;
use std::path::PathBuf;

#[test]
fn test_decorator_with_keyword_args_parses() {
    // Create a minimal test file with keyword decorator arguments
    let test_dir = std::env::temp_dir().join(format!(
        "wj_decorator_kwargs_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Test file with @property_test(iterations=50, seed=42)
    let test_content = r#"
use windjammer_runtime::test::*

@property_test(iterations=50, seed=42)
fn test_random_property(x: i32) {
    assert(x != 0 || x == 0, "Always true")
}
"#;

    fs::write(test_dir.join("prop_test.wj"), test_content).unwrap();

    // Try to parse the file
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("prop_test.wj"))
        .arg("--no-cargo")
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
            "Parser error 'Expected pattern, got Assign' occurred with keyword decorator args:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }
}

