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

/// TDD Test: User-defined functions must shadow built-in runtime mappings
///
/// Bug: The codegen has hardcoded lists of function names (test_macros, test_functions)
/// that get unconditionally redirected to macros or windjammer_runtime::test::* paths.
/// If a user defines their own function with one of these names (e.g., `assert_approx_eq`),
/// the codegen ignores the user's definition and redirects the call anyway.
///
/// Discovered via dogfooding: sdf_test.wj defines a local `assert_approx_eq` helper,
/// but the generated Rust calls `windjammer_runtime::test::assert_approx_eq(...)` instead
/// of the local function, causing an unresolved module error.
///
/// Root Cause: The special-case name checks happen BEFORE consulting the
/// signature_registry, which contains user-defined function information.
///
/// Fix: Check if a function is user-defined (in signature_registry, not extern)
/// before applying any special-case redirects.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_user_defined_assert_approx_not_redirected() {
    // User defines their own assert_approx_eq — it should NOT be redirected
    // to windjammer_runtime::test::assert_approx_eq
    let code = test_utils::compile_single(
        r#"
fn assert_approx_eq(name: string, actual: f32, expected: f32, epsilon: f32) {
    if (actual - expected).abs() < epsilon {
        println("PASS: {}", name)
    } else {
        println("FAIL: {}", name)
    }
}

fn main() {
    assert_approx_eq("test1", 1.0, 1.0, 0.001)
    assert_approx_eq("test2", 2.5, 2.5, 0.001)
}
"#,
    );

    // The generated Rust should call assert_approx_eq() directly, NOT windjammer_runtime::test::assert_approx_eq()
    assert!(
        !code.contains("windjammer_runtime::test::assert_approx_eq"),
        "User-defined assert_approx_eq was redirected to windjammer_runtime! Generated:\n{}",
        code
    );

    // The function definition should exist
    assert!(
        code.contains("fn assert_approx_eq("),
        "User-defined assert_approx_eq function definition missing. Generated:\n{}",
        code
    );

    // Calls should be plain assert_approx_eq(...) (owned name may be String::from / .to_string()).
    assert!(
        code.contains("assert_approx_eq(\"test1\"")
            || code.contains("assert_approx_eq(\"test1\".to_string()")
            || code.contains("assert_approx_eq(String::from(\"test1\")")
            || code.contains("assert_approx_eq(&\"test1\""),
        "Call to assert_approx_eq should be direct, not qualified. Generated:\n{}",
        code
    );
}

#[test]
fn test_user_defined_assert_gt_not_redirected() {
    // Same bug applies to all test_functions: assert_gt, assert_lt, etc.
    let code = test_utils::compile_single(
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
    // When there's NO user-defined assert_approx_eq, it should still redirect
    // to windjammer_runtime::test::assert_approx_eq (existing behavior)
    let code = test_utils::compile_single(
        r#"
fn main() {
    assert_approx_eq(1.0, 1.0, 0.001)
}
"#,
    );

    assert!(
        code.contains("windjammer_runtime::test::assert_approx_eq"),
        "Built-in assert_approx_eq should redirect to windjammer_runtime when not user-defined. Generated:\n{}",
        code
    );
}
