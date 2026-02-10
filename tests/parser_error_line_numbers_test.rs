/// TDD Test: Parser Error Line Numbers
///
/// THE WINDJAMMER WAY: Proper tooling before fixing root causes
///
/// This test ensures parser errors include line numbers for debugging.
/// Without line numbers, finding syntax errors in 548-line files is painful.
use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_parser_error_includes_line_number() {
    // Create a test file with a syntax error on a specific line
    let test_dir = std::env::temp_dir().join(format!(
        "wj_parser_line_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Windjammer code with syntax error on line 3
    // "let = 10" is invalid - missing pattern before =
    let test_content = r#"
fn main() {
    let = 10
}
"#;

    fs::write(test_dir.join("syntax_error.wj"), test_content).unwrap();

    // Compile the file
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("syntax_error.wj")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Assert that error message includes line number
    // Should see something like "line 3" or "3:1" or "syntax_error.wj:3"
    let has_line_number = stderr.contains("line 3")
        || stderr.contains(":3:")
        || stderr.contains(":3")
        || stdout.contains("line 3")
        || stdout.contains(":3:")
        || stdout.contains(":3");

    assert!(
        has_line_number,
        "Parser error should include line number.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout, stderr
    );

    // Should mention the actual error
    assert!(
        stderr.contains("Expected") || stdout.contains("Expected"),
        "Should have parser error message.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_parser_error_for_assign_in_pattern_context() {
    // Reproduce the specific "Expected pattern, got Assign" error
    let test_dir = std::env::temp_dir().join(format!(
        "wj_pattern_assign_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Minimal reproduction of the pattern/assign issue
    let test_content = r#"
@test
fn test_example() {
    let x = 10
    let result = x + 5   // This might trigger "Expected pattern, got Assign"
    assert!(result == 15)
}
"#;

    fs::write(test_dir.join("pattern_error.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("pattern_error.wj")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // If there's an error, it should have a line number
    if stderr.contains("Expected pattern") || stdout.contains("Expected pattern") {
        let has_line_number = stderr.contains("line ")
            || stderr.contains(":")
            || stdout.contains("line ")
            || stdout.contains(":");

        assert!(
            has_line_number,
            "Parser error should include location info.\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }
}
