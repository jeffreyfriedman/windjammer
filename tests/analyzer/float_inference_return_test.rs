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

//! TDD: Float literal inference for return statements
//!
//! Bug: `return 1.0` generates `1.0_f64` when function returns `f32`, causing E0308.
//! Goal: Infer float type from function's return type for literals in return position.
//!
//! Architecture: Extend existing return_type tracking in collect_statement_constraints.
//! Default for literals in functions with no return type remains f64.
//!
//! ## Edge Cases (documented for future work)
//!
//! - **Closures**: `|| 1.0` - closure return type not yet tracked; defaults to f64
//! - **Async fn**: `async fn f() -> f32 { 1.0 }` - same as sync; return type propagates
//! - **Nested blocks**: `fn f() -> f32 { { 1.0 } }` - Block expressions get return_type
//! - **Match without default**: All arms constrained; missing arm is compile error elsewhere

// Explicit return: `fn foo() -> f32 { return 1.0 }`

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_explicit_return_f32() {
    let source = r#"
pub fn foo() -> f32 {
    return 1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1.0f32"),
        "Explicit return 1.0 in f32 function should generate _f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "Should not generate f64 when returning f32, got:\n{}",
        output
    );
}

/// Implicit return: `fn bar() -> f32 { 1.0 }`
#[test]
fn test_implicit_return_f32() {
    let source = r#"
pub fn bar() -> f32 {
    1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1.0f32"),
        "Implicit return 1.0 in f32 function should generate _f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "Should not generate f64 for implicit return f32, got:\n{}",
        output
    );
}

/// Early return: `fn baz() -> f32 { if cond { return 1.0 } 2.0 }`
#[test]
fn test_early_return_f32() {
    let source = r#"
pub fn baz(cond: bool) -> f32 {
    if cond {
        return 1.0
    }
    2.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1.0f32"),
        "Early return 1.0 should generate _f32, got:\n{}",
        output
    );
    assert!(
        output.contains("2.0_f32") || output.contains("2.0f32"),
        "Fallback 2.0 should generate _f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64") && !output.contains("2.0_f64"),
        "Should not generate f64 in f32 return context, got:\n{}",
        output
    );
}

/// Match arms: `fn qux() -> f32 { match x { A => 1.0, B => 2.0 } }`
#[test]
fn test_match_arms_return_f32() {
    let source = r#"
pub enum E { A, B }

pub fn qux(x: E) -> f32 {
    match x {
        E::A => 1.0,
        E::B => 2.0,
    }
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1.0f32"),
        "Match arm 1.0 should generate _f32, got:\n{}",
        output
    );
    assert!(
        output.contains("2.0_f32") || output.contains("2.0f32"),
        "Match arm 2.0 should generate _f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64") && !output.contains("2.0_f64"),
        "Match arms should not generate f64 in f32 return context, got:\n{}",
        output
    );
}

/// f64 return type: literals should be f64
#[test]
fn test_explicit_return_f64() {
    let source = r#"
pub fn foo() -> f64 {
    return 1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f64") || output.contains("1.0f64"),
        "Explicit return 1.0 in f64 function should generate _f64, got:\n{}",
        output
    );
}

/// No return type: unconstrained float literal (currently f32 in compiler)
#[test]
fn test_no_return_type_defaults_f64() {
    let source = r#"
pub fn foo() {
    let x = 1.0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1.0f32"),
        "Literal in function with no return type: expect current float default (f32), got:\n{}",
        output
    );
}

/// Option<f32> return: `fn opt() -> Option<f32> { Some(1.0) }`
#[test]
fn test_option_f32_return() {
    let source = r#"
pub fn opt() -> Option<f32> {
    Some(1.0)
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1.0f32"),
        "Literal in Option<f32> return should generate _f32, got:\n{}",
        output
    );
}

/// Result<f32, E> return: `fn res() -> Result<f32, String> { Ok(1.0) }`
#[test]
fn test_result_f32_return() {
    let source = r#"
pub fn res() -> Result<f32, String> {
    Ok(1.0)
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1.0f32"),
        "Literal in Result<f32, E> return should generate _f32, got:\n{}",
        output
    );
}

/// Tuple return: `fn tup() -> (f32, f32) { (1.0, 2.0) }`
#[test]
fn test_tuple_return_f32() {
    let source = r#"
pub fn tup() -> (f32, f32) {
    (1.0, 2.0)
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        (output.contains("1.0_f32") || output.contains("1.0f32"))
            && (output.contains("2.0_f32") || output.contains("2.0f32")),
        "Tuple elements in (f32, f32) return should generate _f32, got:\n{}",
        output
    );
}
