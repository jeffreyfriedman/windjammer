use std::env;
/// TDD Test: Auto-Mutability Inference
///
/// THE WINDJAMMER WAY: The compiler infers `mut` when mutations are detected
///
/// Windjammer Philosophy:
/// - Infer what doesn't matter (ownership, mutability, simple types)
/// - Compiler does the work, not the developer
/// - No ceremony for obvious cases
///
/// Test: When a binding's fields are mutated, automatically add `mut`
use std::fs;
use std::path::PathBuf;

#[test]
fn test_auto_mut_on_field_mutation() {
    // Create a test file where a struct field is mutated
    let test_dir = std::env::temp_dir().join(format!(
        "wj_auto_mut_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Windjammer code that mutates a field without explicit `mut`
    let test_content = r#"
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let point = Point { x: 0, y: 0 }
    point.x = 10  // THE WINDJAMMER WAY: Compiler infers `mut` here!
    point.y = 20
    println!("Point: ({}, {})", point.x, point.y)
}
"#;

    fs::write(test_dir.join("auto_mut.wj"), test_content).unwrap();

    // Compile the file (from temp directory so output goes there)
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir) // Run from temp directory
        .arg("build")
        .arg("auto_mut.wj") // Use relative path
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    // Read generated Rust code
    let rust_file = test_dir.join("build").join("auto_mut.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_default();

    // Debug output if needed
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Assert that `mut` was automatically added
    assert!(
        rust_code.contains("let mut point =") || rust_code.contains("let mut point="),
        "Should automatically add 'mut' when field is mutated.\nGenerated code:\n{}",
        rust_code
    );

    // Should compile successfully with rustc (stdout/stderr already printed above)
    assert!(
        !stderr.contains("cannot mutate") && !stdout.contains("cannot mutate"),
        "Should not have mutability errors.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}

#[test]
fn test_no_mut_when_not_mutated() {
    // Create a test file where no mutation occurs
    let test_dir = std::env::temp_dir().join(format!(
        "wj_no_mut_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Windjammer code that doesn't mutate
    let test_content = r#"
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let point = Point { x: 10, y: 20 }
    println!("Point: ({}, {})", point.x, point.y)
}
"#;

    fs::write(test_dir.join("no_mut.wj"), test_content).unwrap();

    // Compile the file (from temp directory so output goes there)
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let _output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir) // Run from temp directory
        .arg("build")
        .arg("no_mut.wj") // Use relative path
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    // Read generated Rust code
    let rust_file = test_dir.join("build").join("no_mut.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_default();

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Assert that `mut` was NOT added (no mutation occurred)
    assert!(
        !rust_code.contains("let mut point =") && !rust_code.contains("let mut point="),
        "Should NOT add 'mut' when no mutation occurs.\nGenerated code:\n{}",
        rust_code
    );
}
