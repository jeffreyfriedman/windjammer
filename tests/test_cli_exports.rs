//! TDD: Verify CLI functions are exported from windjammer lib.
//!
//! These functions are used by cli/build.rs and cli/test.rs when the cli feature is enabled.
//! They must be accessible via windjammer:: for the library to build.

#[cfg(feature = "cli")]
#[test]
fn test_run_tests_is_exported() {
    // Verify run_tests exists and can be called - use temp dir with no .wj files
    let temp = tempfile::tempdir().unwrap();
    let result = windjammer::run_tests(
        Some(temp.path()),
        None,
        false,
        true,
        false,
    );
    // Empty dir: no test files found, returns Ok(())
    assert!(result.is_ok(), "run_tests should succeed for empty dir: {:?}", result);
}

#[cfg(feature = "cli")]
#[test]
fn test_generate_mod_file_is_exported() {
    let temp = tempfile::tempdir().unwrap();
    // Empty dir - should generate nothing but not panic
    let result = windjammer::generate_mod_file(temp.path());
    assert!(result.is_ok(), "generate_mod_file should be callable");
}

#[cfg(feature = "cli")]
#[test]
fn test_strip_main_functions_is_exported() {
    let temp = tempfile::tempdir().unwrap();
    // Empty dir - should do nothing but not panic
    let result = windjammer::strip_main_functions(temp.path());
    assert!(result.is_ok(), "strip_main_functions should be callable");
}
