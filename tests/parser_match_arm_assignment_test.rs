/// TDD Test: Match Arm with Assignment Expression
///
/// THE WINDJAMMER WAY: Fix the root cause with TDD
///
/// Bug: Parser incorrectly parses assignment expressions in match arms as patterns
/// Should be: Pattern => Expression (where Expression can be an assignment)
/// Was doing: Pattern => Pattern (incorrectly)
use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_match_arm_with_assignment() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_match_assign_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Minimal reproduction of the issue
    let test_content = r#"
enum NodeType {
    End,
    Start
}

fn main() {
    let node_type = NodeType::End
    let mut has_end = false
    match node_type {
        NodeType::End => has_end = true,
        _ => {}
    }
}
"#;

    fs::write(test_dir.join("match_test.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("match_test.wj")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Should compile successfully (NOT "Expected pattern, got Assign")
    assert!(
        output.status.success(),
        "Match arm with assignment should compile.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );

    assert!(
        !stderr.contains("Expected pattern") && !stdout.contains("Expected pattern"),
        "Should not have pattern error for assignment expression.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}

#[test]
fn test_match_arm_with_complex_assignment() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_match_complex_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Test with more complex assignments (field access, compound, etc.)
    let test_content = r#"
struct Data {
    value: i32
}

enum Action {
    Increment,
    Decrement
}

fn main() {
    let action = Action::Increment
    let mut data = Data { value: 0 }
    
    match action {
        Action::Increment => data.value = data.value + 1,
        Action::Decrement => data.value = data.value - 1
    }
}
"#;

    fs::write(test_dir.join("complex_match.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("complex_match.wj")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Should compile successfully
    assert!(
        output.status.success(),
        "Match arm with complex assignment should compile.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}
