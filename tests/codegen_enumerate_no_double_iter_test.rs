/// TDD Test: .enumerate() should not produce double .iter().iter().enumerate()
///
/// Bug: When Windjammer source has `items.iter().enumerate()`, the codegen
/// processes `.iter()` first (producing `items.iter()`), then sees `.enumerate()`
/// and blindly wraps it with `.iter().enumerate()`, resulting in
/// `items.iter().iter().enumerate()` which is incorrect.
///
/// The fix: if the object already ends with `.iter()`, `.iter_mut()`, or
/// `.into_iter()`, skip adding the extra `.iter()` prefix.
///
/// Discovered via: codegen_loops_comprehensive_tests::test_enumerate_basic
fn compile_to_rust(source: &str) -> String {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    std::fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    std::fs::create_dir_all(&output_dir).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !output.status.success() {
        panic!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generated = output_dir.join("test.rs");
    std::fs::read_to_string(&generated).unwrap_or_else(|_| {
        panic!(
            "No test.rs generated. stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn test_iter_enumerate_no_double_iter() {
    // items.iter().enumerate() should NOT produce items.iter().iter().enumerate()
    let code = compile_to_rust(
        r#"
fn main() {
    let items = vec![10, 20, 30]
    for (i, item) in items.iter().enumerate() {
        println("{}: {}", i, item)
    }
}
"#,
    );

    // Should NOT contain .iter().iter()
    assert!(
        !code.contains(".iter().iter()"),
        "Double .iter().iter() detected! Generated:\n{}",
        code
    );

    // Should contain .iter().enumerate() (single .iter())
    assert!(
        code.contains(".iter().enumerate()"),
        "Expected .iter().enumerate() in output. Generated:\n{}",
        code
    );
}

#[test]
fn test_vec_enumerate_adds_single_iter() {
    // items.enumerate() (no explicit .iter()) should produce items.iter().enumerate()
    let code = compile_to_rust(
        r#"
fn main() {
    let items = vec![10, 20, 30]
    for (i, item) in items.enumerate() {
        println("{}: {}", i, item)
    }
}
"#,
    );

    // Should contain exactly one .iter().enumerate()
    assert!(
        code.contains(".iter().enumerate()"),
        "Expected .iter().enumerate() for bare .enumerate(). Generated:\n{}",
        code
    );

    // Should NOT contain .iter().iter()
    assert!(
        !code.contains(".iter().iter()"),
        "Double .iter().iter() detected! Generated:\n{}",
        code
    );
}
