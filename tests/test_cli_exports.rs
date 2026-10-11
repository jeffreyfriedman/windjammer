#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: Verify CLI functions are exported from windjammer lib.
//!
//! These functions are used by cli/build.rs and cli/test.rs when the cli feature is enabled.
//! They must be accessible via windjammer:: for the library to build.

#[cfg(feature = "cli")]
#[test]
fn test_run_tests_is_exported() {
    // Finding zero tests is an error by default (a silently green empty run hides misconfiguration)...
    let temp = tempfile::tempdir().unwrap();
    let result = windjammer::run_tests(Some(temp.path()), None, false, true, false);
    assert!(result.is_err(), "empty dir must be an error by default: {:?}", result);

    // ...and `--allow-empty` (exported `TestRunOptions`) opts back in to success.
    let opts = windjammer::TestRunOptions {
        path: Some(temp.path().to_path_buf()),
        allow_empty: true,
        ..Default::default()
    };
    let result = windjammer::run_tests_with_options(opts);
    assert!(result.is_ok(), "allow_empty should succeed for empty dir: {:?}", result);
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
