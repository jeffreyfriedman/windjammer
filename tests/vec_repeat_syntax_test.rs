/// Test: vec![value; count] syntax should be preserved
///
/// Bug #7: Transpiler incorrectly converts semicolon to comma in vec! macro
///
/// Example:
/// ```
/// vec![0; 5]          // Create vec with 5 copies of 0
/// vec![None; 10]      // Create vec with 10 None values
/// vec![vec![0; 3]; 2] // Create 2D vec: 2 rows of 3 zeros
/// ```
///
/// The transpiler was converting:
/// ```
/// vec![0; 5]       → vec![0, 5]        ❌ WRONG (creates [0, 5])
/// vec![None; 10]   → vec![None, 10]    ❌ WRONG (doesn't compile)
/// ```
///
/// This causes type mismatches when the repeated value and count have different types.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_vec_repeat_simple() {
    let source = r#"
pub fn test_repeat() {
    let v: Vec<i32> = vec![0; 5];
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    // Should preserve semicolon, not convert to comma
    assert!(
        generated.contains("vec![0; 5]"),
        "Expected 'vec![0; 5]' but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("vec![0, 5]"),
        "Should NOT generate 'vec![0, 5]', found in:\n{}",
        generated
    );
}

#[test]
fn test_vec_repeat_option() {
    let source = r#"
pub enum MyEnum {
    A,
    B,
}

pub fn test_option_repeat() {
    let v: Vec<Option<MyEnum>> = vec![None; 10];
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    // Should preserve semicolon
    assert!(
        generated.contains("vec![None; 10]"),
        "Expected 'vec![None; 10]' but got:\n{}",
        generated
    );
}

#[test]
fn test_vec_repeat_nested() {
    let source = r#"
pub fn test_2d_vec() {
    let v: Vec<Vec<i32>> = vec![vec![0; 3]; 2];
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    // Should preserve both semicolons
    assert!(
        generated.contains("vec![vec![0; 3]; 2]"),
        "Expected 'vec![vec![0; 3]; 2]' but got:\n{}",
        generated
    );

    // Should NOT have commas
    assert!(
        !generated.contains("vec![vec![0, 3], 2]"),
        "Should NOT generate 'vec![vec![0, 3], 2]', found in:\n{}",
        generated
    );
}

#[test]
fn test_vec_repeat_complex_value() {
    let source = r#"
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub fn test_struct_repeat() {
    let origin = Point { x: 0, y: 0 };
    let v: Vec<Point> = vec![origin; 5];
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    // Should preserve semicolon with identifier
    assert!(
        generated.contains("vec![origin; 5]") || generated.contains("vec![origin.clone(); 5]"),
        "Expected 'vec![origin; 5]' or 'vec![origin.clone(); 5]' but got:\n{}",
        generated
    );
}
