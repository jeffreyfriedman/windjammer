use std::env;
/// TDD Test: Decorator with Expression Arguments
///
/// THE WINDJAMMER WAY: Fix the root cause, not symptoms
///
/// Bug: Parser fails with "Expected pattern, got Assign" on decorators like:
/// - @requires(id.len() > 0)
/// - @ensures(result.id == id)
/// - @property_test(iterations=50, seed=42)
///
/// These contain expressions with operators (>, ==, =) which the parser
/// is incorrectly trying to parse as patterns.
use std::fs;
use std::path::PathBuf;

#[test]
fn test_requires_decorator_with_expression() {
    // Create a test file with @requires containing an expression
    let test_dir = std::env::temp_dir().join(format!(
        "wj_requires_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let test_content = r#"
@requires(id.len() > 0 && text.len() > 0)
fn create_node(id: string, text: string) -> string {
    format!("{}: {}", id, text)
}

fn main() {
    let node = create_node("node1", "Hello")
    println!("{}", node)
}
"#;

    fs::write(test_dir.join("requires_test.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("requires_test.wj"))
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Should NOT have "Expected pattern, got Assign" error
    assert!(
        !stderr.contains("Expected pattern") && !stdout.contains("Expected pattern"),
        "Should parse @requires decorator with expressions.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );

    // Should compile successfully
    assert!(
        output.status.success(),
        "Should compile successfully.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}

#[test]
fn test_ensures_decorator_with_expression() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_ensures_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let test_content = r#"
@ensures(result.len() > 0)
fn create_message(text: string) -> string {
    format!("Message: {}", text)
}

fn main() {
    let msg = create_message("Hello")
    println!("{}", msg)
}
"#;

    fs::write(test_dir.join("ensures_test.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("ensures_test.wj"))
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    assert!(
        !stderr.contains("Expected pattern") && !stdout.contains("Expected pattern"),
        "Should parse @ensures decorator with expressions.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );

    assert!(
        output.status.success(),
        "Should compile successfully.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}
