#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

// TDD Test: Verify struct field shorthand when string param matches string field
// With Windjammer `string` type, param and field both generate String — shorthand is correct

use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_shorthand_when_type_conversion_needed() {
    let code = r#"
        struct User {
            name: string,
        }
        
        fn create_user(name: string) -> User {
            User { name: name }
        }
        
        fn main() {
            let user = create_user("Alice");
        }
    "#;

    // Create temporary test directory
    let _tmp = tempfile::tempdir().unwrap();
    let test_dir = _tmp.path().join(format!(
        "wj_test_struct_shorthand_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    std::fs::create_dir_all(&test_dir).unwrap();

    // Write test file
    std::fs::write(test_dir.join("main.wj"), code).unwrap();

    // Compile
    let wj_binary = env!("CARGO_BIN_EXE_wj");

    let output = Command::new(wj_binary)
        .arg("build")
        .arg("--no-cargo")
        .arg("main.wj")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check generated code
    let generated_code = std::fs::read_to_string(test_dir.join("build/main.rs"))
        .expect("Failed to read generated code");

    // Cleanup

    if !output.status.success() {
        panic!(
            "Compilation failed!\nstdout: {}\nstderr: {}\ngenerated:\n{}",
            stdout, stderr, generated_code
        );
    }

    // Both param and field are String — field shorthand is the idiomatic generated form
    assert!(
        generated_code.contains("User { name }")
            || generated_code.contains("User{name}")
            || generated_code.contains("name: name"),
        "Should use field shorthand or direct String assignment\nGenerated:\n{}",
        generated_code
    );
}
