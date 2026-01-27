/// TDD Test: Decorator Expression Generation
/// Test that @requires and @ensures decorators correctly handle various expressions
use std::fs;
use std::path::PathBuf;

fn compile_and_check(source: &str) -> String {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_decorator_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let test_file = test_dir.join("test.wj");
    fs::write(&test_file, source).unwrap();

    // Compile using wj
    let output = std::process::Command::new("wj")
        .args(&["build", test_file.to_str().unwrap(), "--no-cargo"])
        .current_dir(&test_dir)
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read generated Rust file
    let rust_file = test_dir.join("build").join("test.rs");
    let result = fs::read_to_string(&rust_file).expect("Failed to read generated Rust");

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    result
}

#[test]
fn test_requires_simple_comparison() {
    let source = r#"
@requires(amount > 0)
fn add_gold(amount: i32) {
}
"#;

    let result = compile_and_check(source);

    // Should generate proper runtime check
    assert!(
        result.contains("amount > 0"),
        "Should preserve simple comparison"
    );
    assert!(
        !result.contains("/* expression */"),
        "Should not have placeholder"
    );
}

#[test]
fn test_ensures_field_access() {
    let source = r#"
@ensures(result.value >= 0)
fn get_value() -> Point {
    Point { value: 5 }
}

struct Point {
    pub value: i32
}
"#;

    let result = compile_and_check(source);

    // Should handle field access on result
    assert!(
        result.contains("__result.value >= 0"),
        "Should handle field access with result replacement"
    );
    assert!(
        !result.contains("/* expression */"),
        "Should not have placeholder"
    );
}

#[test]
fn test_requires_method_call() {
    let source = r#"
@requires(name.len() > 0)
fn process(name: string) {
}
"#;

    let result = compile_and_check(source);

    // Should handle method calls
    assert!(
        result.contains("name.len() > 0"),
        "Should handle method calls"
    );
    assert!(
        !result.contains("/* expression */"),
        "Should not have placeholder"
    );
}

#[test]
fn test_requires_complex_expression() {
    let source = r#"
@requires(level >= 1 && level <= 100)
fn set_level(level: i32) {
}
"#;

    let result = compile_and_check(source);

    // Should handle complex boolean expressions
    assert!(
        result.contains("level >= 1 && level <= 100"),
        "Should handle complex expressions"
    );
    assert!(
        !result.contains("/* expression */"),
        "Should not have placeholder"
    );
}
