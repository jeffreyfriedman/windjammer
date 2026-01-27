use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Test that @ensures can access parameters even when they're moved in the function body
///
/// Bug: Parameters are moved in function body, then accessed in @ensures, causing E0382
/// Fix: Clone parameters before function body when @ensures references them
#[test]
fn test_ensures_can_access_moved_parameters() {
    let source = r#"
struct User {
    pub name: string,
    pub age: i32,
}

@requires(name.len() > 0)
@ensures(result.name == name)
fn create_user(name: string, age: i32) -> User {
    User { name: name, age: age }
}

@test
fn test_create_user() {
    let user = create_user("Alice", 25)
    assert_eq(user.name, "Alice")
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test.wj");
    fs::write(&input_path, source).unwrap();

    // Build with wj
    let output = Command::new("wj")
        .args(["build", input_path.to_str().unwrap(), "--no-cargo"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        panic!(
            "Windjammer compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Read generated Rust code
    let rust_path = temp_dir.path().join("build/test.rs");
    let rust_code = fs::read_to_string(&rust_path).unwrap();

    println!("Generated Rust:\n{}", rust_code);

    // Strip out windjammer_runtime calls for rustc test
    let rust_code_for_rustc = rust_code
        .replace("windjammer_runtime::test::requires", "let _ = ")
        .replace("windjammer_runtime::test::ensures", "let _ = ");

    let rustc_test_path = temp_dir.path().join("test_rustc.rs");
    fs::write(&rustc_test_path, &rust_code_for_rustc).unwrap();

    // Verify the Rust code compiles (or fails with E0382 if bug not fixed)
    let rustc_output = Command::new("rustc")
        .args([
            rustc_test_path.to_str().unwrap(),
            "--crate-type",
            "lib",
            "--test",
            "-o",
            temp_dir.path().join("test_binary").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);

        // The bug manifests as E0382: borrow of moved value
        if stderr.contains("E0382") {
            panic!(
                "BUG CONFIRMED: E0382 borrow of moved value in @ensures!\n\nRust code:\n{}\n\nRustc errors:\n{}",
                rust_code, stderr
            );
        }

        panic!(
            "Generated Rust code failed to compile!\n\nRust code:\n{}\n\nRustc errors:\n{}",
            rust_code, stderr
        );
    }

    // SUCCESS: The code compiles, meaning the bug is fixed!
    // Verify the fix: parameters should be cloned before function body
    assert!(
        rust_code.contains("__name_for_ensures") || rust_code.contains("name.clone()"),
        "Expected parameter to be cloned for @ensures access"
    );
}

#[test]
fn test_ensures_with_multiple_moved_parameters() {
    let source = r#"
struct Point {
    pub x: i32,
    pub y: i32,
}

@requires(label.len() > 0)
@ensures(result.x == x && result.y == y)
fn create_point(label: string, x: i32, y: i32) -> Point {
    Point { x: x, y: y }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("test.wj");
    fs::write(&input_path, source).unwrap();

    let output = Command::new("wj")
        .args(["build", input_path.to_str().unwrap(), "--no-cargo"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        panic!(
            "Windjammer compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let rust_path = temp_dir.path().join("build/test.rs");
    let rust_code = fs::read_to_string(&rust_path).unwrap();

    // Strip out windjammer_runtime calls for rustc test
    let rust_code_for_rustc = rust_code
        .replace("windjammer_runtime::test::requires", "let _ = ")
        .replace("windjammer_runtime::test::ensures", "let _ = ");

    let rustc_test_path = temp_dir.path().join("test_rustc.rs");
    fs::write(&rustc_test_path, &rust_code_for_rustc).unwrap();

    // Verify it compiles with rustc
    let rustc_output = Command::new("rustc")
        .args([
            rustc_test_path.to_str().unwrap(),
            "--crate-type",
            "lib",
            "-o",
            temp_dir.path().join("test_binary").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Generated Rust code failed to compile!\n\nRustc errors:\n{}",
            stderr
        );
    }
}
