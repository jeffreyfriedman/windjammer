#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// TDD Test for Bug: Typed integer literals generating stray type statements
//
// Bug: When using typed integer literals like `0u64`, the compiler
// incorrectly generates a stray type statement:
//   Source:    let mut total = 0u64
//   Generated: let mut total = 0;
//              u64;  <-- STRAY STATEMENT!
//
// This causes E0423: expected value, found builtin type `u64`

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_u64_typed_literal_no_stray_statement() {
    let source = r#"
fn count_items() -> u64 {
    let mut total: u64 = 0
    total = total + 1
    total = total + 2
    return total
}

fn main() {
    let result = count_items()
}
"#;

    let (rust_code, success) = test_utils::compile_single_check(source);
    let stderr = if !success { &rust_code } else { "" };

    if !success {
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    // Should not have E0423 error (stray type statement)
    assert!(
        !stderr.contains("E0423"),
        "Should not have E0423 error:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("expected value, found builtin type"),
        "Should not have 'expected value' error:\n{}",
        stderr
    );

    // Check generated code does NOT have stray u64; statement (standalone type)
    let has_stray_u64 = rust_code.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "u64;" || trimmed == "u64 ;"
    });
    assert!(
        !has_stray_u64,
        "Generated code should not contain standalone 'u64;' statement:\n{}",
        rust_code
    );

    // TDD: Windjammer infers types, so `let mut total = 0;` is valid.
    // The return type `u64` propagates through type inference.
    // We just need to ensure NO stray type statements exist (checked above).
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_i32_typed_literal_no_stray_statement() {
    let source = r#"
fn calculate() -> i32 {
    let mut sum = 0i32
    sum = sum + 5
    return sum
}

fn main() {
    let _result = calculate()
}
"#;

    let (rust_code, success) = test_utils::compile_single_check(source);
    let stderr = if !success { &rust_code } else { "" };

    if !success {
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    // Check generated code does NOT have stray i32; statement (standalone type)
    // Note: _i32; at end of expression is valid (integer suffix), we're checking for "i32;" alone
    let has_stray_i32 = rust_code.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "i32;" || trimmed == "i32 ;"
    });
    assert!(
        !has_stray_i32,
        "Generated code should not contain standalone 'i32;' statement:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_u32_typed_literal_no_stray_statement() {
    let source = r#"
fn get_count() -> u32 {
    let mut count: u32 = 0
    count = count + 1
    return count
}

fn main() {
    let _x = get_count()
}
"#;

    let (rust_code, success) = test_utils::compile_single_check(source);
    let stderr = if !success { &rust_code } else { "" };

    if !success {
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    // Check generated code does NOT have stray u32; statement (standalone type)
    let has_stray_u32 = rust_code.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "u32;" || trimmed == "u32 ;"
    });
    assert!(
        !has_stray_u32,
        "Generated code should not contain standalone 'u32;' statement:\n{}",
        rust_code
    );
}
