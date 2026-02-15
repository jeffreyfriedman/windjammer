/// TDD Test: User-defined functions must shadow built-in runtime mappings
///
/// Bug: The codegen has hardcoded lists of function names (test_macros, test_functions)
/// that get unconditionally redirected to macros or windjammer_runtime::test::* paths.
/// If a user defines their own function with one of these names (e.g., `assert_approx`),
/// the codegen ignores the user's definition and redirects the call anyway.
///
/// Discovered via dogfooding: sdf_test.wj defines a local `assert_approx` helper,
/// but the generated Rust calls `windjammer_runtime::test::assert_approx(...)` instead
/// of the local function, causing an unresolved module error.
///
/// Root Cause: The special-case name checks happen BEFORE consulting the
/// signature_registry, which contains user-defined function information.
///
/// Fix: Check if a function is user-defined (in signature_registry, not extern)
/// before applying any special-case redirects.

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
fn test_user_defined_assert_approx_not_redirected() {
    // User defines their own assert_approx — it should NOT be redirected
    // to windjammer_runtime::test::assert_approx
    let code = compile_to_rust(
        r#"
fn assert_approx(name: &str, actual: f32, expected: f32, epsilon: f32) {
    if (actual - expected).abs() < epsilon {
        println("PASS: {}", name)
    } else {
        println("FAIL: {}", name)
    }
}

fn main() {
    assert_approx("test1", 1.0, 1.0, 0.001)
    assert_approx("test2", 2.5, 2.5, 0.001)
}
"#,
    );

    // The generated Rust should call assert_approx() directly, NOT windjammer_runtime::test::assert_approx()
    assert!(
        !code.contains("windjammer_runtime::test::assert_approx"),
        "User-defined assert_approx was redirected to windjammer_runtime! Generated:\n{}",
        code
    );

    // The function definition should exist
    assert!(
        code.contains("fn assert_approx("),
        "User-defined assert_approx function definition missing. Generated:\n{}",
        code
    );

    // Calls should be plain assert_approx(...)
    assert!(
        code.contains("assert_approx(\"test1\"")
            || code.contains("assert_approx(\"test1\".to_string()")
            || code.contains("assert_approx(&\"test1\""),
        "Call to assert_approx should be direct, not qualified. Generated:\n{}",
        code
    );
}

#[test]
fn test_user_defined_assert_gt_not_redirected() {
    // Same bug applies to all test_functions: assert_gt, assert_lt, etc.
    let code = compile_to_rust(
        r#"
fn assert_gt(a: f32, b: f32) {
    if a > b {
        println("OK")
    } else {
        println("FAIL")
    }
}

fn main() {
    assert_gt(5.0, 3.0)
}
"#,
    );

    assert!(
        !code.contains("windjammer_runtime::test::assert_gt"),
        "User-defined assert_gt was redirected to windjammer_runtime! Generated:\n{}",
        code
    );

    assert!(
        code.contains("fn assert_gt("),
        "User-defined assert_gt function definition missing. Generated:\n{}",
        code
    );
}

#[test]
fn test_builtin_assert_approx_still_redirected_when_not_user_defined() {
    // When there's NO user-defined assert_approx, it should still redirect
    // to windjammer_runtime::test::assert_approx (existing behavior)
    let code = compile_to_rust(
        r#"
fn main() {
    assert_approx(1.0, 1.0, 0.001)
}
"#,
    );

    assert!(
        code.contains("windjammer_runtime::test::assert_approx"),
        "Built-in assert_approx should redirect to windjammer_runtime when not user-defined. Generated:\n{}",
        code
    );
}
