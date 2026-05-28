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

/// TDD Test: Float inference for constant-folded expressions
///
/// Bug: 1.0 / 60.0 in struct literal generates f64 even when field is f32
/// Pattern: Binary operation gets constant-folded, but the folded value isn't constrained
/// Expected: Folded literal should match the struct field type (f32)
///
/// Example from windjammer-game:
/// ```windjammer
/// pub struct GameConfig {
///     pub timestep: f32,
/// }
/// pub fn create() -> GameConfig {
///     GameConfig { timestep: 1.0 / 60.0 }  // Should be f32, not f64
/// }
/// ```
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_const_fold_in_struct_literal() {
    let source = r#"pub struct Config {
    pub timestep: f32,
}

pub fn create() -> Config {
    Config {
        timestep: 1.0 / 60.0,
    }
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The constant-folded value should be f32
    assert!(
        output.contains("_f32") && !output.contains("_f64"),
        "Expected '_f32' suffix (not '_f64') in generated code:\n{}",
        output
    );
}

#[test]
fn test_const_fold_simple() {
    let source = r#"pub fn compute() -> f32 {
    1.0 / 2.0
}
"#;

    let output = test_utils::compile_single(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    // Return type is f32, so folded value should be f32
    assert!(output.contains("_f32"), "Expected '_f32' in generated code");
    assert!(
        !output.contains("_f64"),
        "Should not contain '_f64':\n{}",
        output
    );
}

// Helper function to compile Windjammer source and get generated Rust
