// Pattern Matching Tests
// Automated tests for pattern matching features

use std::process::Command;
use std::fs;
use std::path::PathBuf;

fn get_wj_compiler() -> PathBuf {
    // Use the release build of the compiler
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj")
}

fn compile_wj_code(code: &str) -> Result<String, String> {
    let temp_file = "/tmp/test_pattern_matching.wj";
    fs::write(temp_file, code).expect("Failed to write test file");
    
    let output = Command::new(get_wj_compiler())
        .args(&["build", temp_file, "--no-cargo"])
        .output()
        .expect("Failed to execute compiler");
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn compile_should_succeed(code: &str, test_name: &str) {
    match compile_wj_code(code) {
        Ok(_) => println!("✓ {} passed", test_name),
        Err(e) => panic!("✗ {} failed: {}", test_name, e),
    }
}

fn compile_should_fail(code: &str, expected_error: &str, test_name: &str) {
    match compile_wj_code(code) {
        Ok(_) => panic!("✗ {} should have failed but succeeded", test_name),
        Err(e) => {
            if e.contains(expected_error) {
                println!("✓ {} passed (correctly rejected)", test_name);
            } else {
                panic!("✗ {} failed with wrong error.\nExpected: {}\nGot: {}", 
                       test_name, expected_error, e);
            }
        }
    }
}

// ============================================================================
// TEST 1: Tuple Enum Variants - Definition
// ============================================================================

#[test]
fn test_tuple_enum_definition_single_field() {
    let code = r#"
enum Option<T> {
    Some(T),
    None,
}
"#;
    compile_should_succeed(code, "tuple_enum_single_field");
}

#[test]
fn test_tuple_enum_definition_multiple_fields() {
    let code = r#"
enum Color {
    Rgb(i32, i32, i32),
    Rgba(i32, i32, i32, i32),
}
"#;
    compile_should_succeed(code, "tuple_enum_multiple_fields");
}

#[test]
fn test_tuple_enum_definition_mixed() {
    let code = r#"
enum Shape {
    Circle(f32),
    Rectangle(f32, f32),
    Point,
}
"#;
    compile_should_succeed(code, "tuple_enum_mixed");
}

// ============================================================================
// TEST 2: Tuple Enum Variants - Pattern Matching
// ============================================================================

#[test]
fn test_tuple_enum_match_single_binding() {
    let code = r#"
enum Option<T> {
    Some(T),
    None,
}

fn unwrap(opt: Option<i32>) -> i32 {
    match opt {
        Option::Some(x) => { return x }
        Option::None => { return 0 }
    }
}
"#;
    compile_should_succeed(code, "tuple_enum_match_single");
}

#[test]
fn test_tuple_enum_match_multiple_bindings() {
    let code = r#"
enum Color {
    Rgb(i32, i32, i32),
}

fn sum_rgb(color: Color) -> i32 {
    match color {
        Color::Rgb(r, g, b) => { return r + g + b }
    }
}
"#;
    compile_should_succeed(code, "tuple_enum_match_multiple");
}

#[test]
fn test_tuple_enum_match_wildcards() {
    let code = r#"
enum Color {
    Rgb(i32, i32, i32),
}

fn get_red(color: Color) -> i32 {
    match color {
        Color::Rgb(r, _, _) => { return r }
    }
}
"#;
    compile_should_succeed(code, "tuple_enum_match_wildcards");
}

// ============================================================================
// TEST 3: Let Patterns - Irrefutable (Should Work)
// ============================================================================

#[test]
fn test_let_tuple_destructuring() {
    let code = r#"
fn test() -> i32 {
    let (x, y) = (10, 20)
    return x + y
}
"#;
    compile_should_succeed(code, "let_tuple_destructuring");
}

#[test]
fn test_let_nested_tuple_destructuring() {
    let code = r#"
fn test() -> i32 {
    let ((a, b), (c, d)) = ((1, 2), (3, 4))
    return a + b + c + d
}
"#;
    compile_should_succeed(code, "let_nested_tuple");
}

#[test]
fn test_let_wildcard() {
    let code = r#"
fn test() -> i32 {
    let _ = 100
    let (x, _) = (10, 20)
    return x
}
"#;
    compile_should_succeed(code, "let_wildcard");
}

// ============================================================================
// TEST 4: Let Patterns - Refutable (Should Fail)
// ============================================================================

#[test]
fn test_let_enum_variant_rejected() {
    let code = r#"
enum Option<T> {
    Some(T),
    None,
}

fn test() -> i32 {
    let opt = Option::Some(42)
    let Option::Some(x) = opt
    return x
}
"#;
    compile_should_fail(code, "Refutable pattern", "let_enum_variant_rejected");
}

#[test]
fn test_let_literal_rejected() {
    let code = r#"
fn test() -> i32 {
    let x = 42
    let 42 = x
    return x
}
"#;
    compile_should_fail(code, "Refutable pattern", "let_literal_rejected");
}

// ============================================================================
// TEST 5: Consistency - Number Literals
// ============================================================================

#[test]
fn test_hex_literals() {
    let code = r#"
fn test() -> i64 {
    let x = 0xFF
    let y = 0xDEADBEEF
    return x + y
}
"#;
    compile_should_succeed(code, "hex_literals");
}

#[test]
fn test_binary_literals() {
    let code = r#"
fn test() -> i64 {
    let x = 0b1010
    let y = 0b1111_0000
    return x + y
}
"#;
    compile_should_succeed(code, "binary_literals");
}

#[test]
fn test_octal_literals() {
    let code = r#"
fn test() -> i64 {
    let x = 0o755
    let y = 0o644
    return x + y
}
"#;
    compile_should_succeed(code, "octal_literals");
}

// ============================================================================
// TEST 6: Consistency - Module Paths
// ============================================================================

#[test]
fn test_module_path_double_colon() {
    let code = r#"
use std::fs::File
"#;
    compile_should_succeed(code, "module_path_double_colon");
}

#[test]
fn test_module_path_slash_rejected() {
    let code = r#"
use std/fs
"#;
    compile_should_fail(code, "Use '::'", "module_path_slash_rejected");
}

#[test]
fn test_module_path_dot_rejected() {
    let code = r#"
use std.fs
"#;
    compile_should_fail(code, "Use '::'", "module_path_dot_rejected");
}

// ============================================================================
// TEST 7: Consistency - Qualified Paths
// ============================================================================

#[test]
fn test_qualified_path_in_type() {
    let code = r#"
struct Event {
    pub value: i32,
}
"#;
    compile_should_succeed(code, "qualified_path_in_type");
}

#[test]
fn test_qualified_path_in_match() {
    let code = r#"
enum Color {
    Red,
    Green,
}

fn test(c: Color) -> i32 {
    match c {
        Color::Red => { return 1 }
        Color::Green => { return 2 }
    }
}
"#;
    compile_should_succeed(code, "qualified_path_in_match");
}

// ============================================================================
// MAIN TEST RUNNER
// ============================================================================

#[test]
fn run_all_pattern_tests() {
    println!("\n=== Running Pattern Matching Tests ===\n");
    
    // Note: Individual tests run via cargo test
    // This is just a summary
    println!("Run with: cargo test --test pattern_matching_tests");
}

