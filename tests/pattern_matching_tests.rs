// Pattern Matching Tests
// Automated tests for pattern matching features

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    // Use the release build of the compiler
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj")
}

fn compile_wj_code(code: &str) -> Result<String, String> {
    // Use unique temp file to avoid test interference
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_file = format!("/tmp/test_pattern_matching_{}.wj", timestamp);

    fs::write(&temp_file, code).expect("Failed to write test file");

    let output = Command::new(get_wj_compiler())
        .args(["build", &temp_file, "--no-cargo"])
        .output()
        .expect("Failed to execute compiler");

    // Clean up temp file
    let _ = fs::remove_file(&temp_file);

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        // Capture both stdout and stderr for error messages
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{}{}", stdout, stderr))
    }
}

fn compile_should_succeed(code: &str, test_name: &str) {
    match compile_wj_code(code) {
        Ok(_) => println!("✓ {} passed", test_name),
        Err(e) => panic!("✗ {} failed: {}", test_name, e),
    }
}

fn compile_and_check_rust_compiles(wj_file: &str, test_name: &str) {
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(wj_file);

    // First, compile the Windjammer code
    let output = Command::new(get_wj_compiler())
        .args(["build", wj_path.to_str().unwrap(), "--no-cargo"])
        .output()
        .expect("Failed to execute compiler");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "✗ {} failed to compile Windjammer: {}{}",
            test_name, stdout, stderr
        );
    }

    // Get the generated Rust file name (remove .wj extension, add .rs)
    let rust_file = wj_file.replace(".wj", ".rs");
    let rust_path = PathBuf::from("./build").join(&rust_file);

    // Then, try to compile the generated Rust code with rustc
    // Use --crate-type=lib to avoid needing a main function
    let rust_output = Command::new("rustc")
        .args([
            rust_path.to_str().unwrap(),
            "--crate-type=lib",
            "--edition=2021",
            "-O",
        ])
        .output()
        .expect("Failed to execute rustc");

    if !rust_output.status.success() {
        let stdout = String::from_utf8_lossy(&rust_output.stdout);
        let stderr = String::from_utf8_lossy(&rust_output.stderr);
        panic!(
            "✗ {} failed to compile Rust: {}{}",
            test_name, stdout, stderr
        );
    }

    println!("✓ {} passed (Windjammer + Rust compilation)", test_name);
}

fn compile_and_check_generated_rust(wj_file: &str, expected_imports: &[&str], test_name: &str) {
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(wj_file);

    let output = Command::new(get_wj_compiler())
        .args(["build", wj_path.to_str().unwrap(), "--no-cargo"])
        .output()
        .expect("Failed to execute compiler");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("✗ {} failed to compile: {}{}", test_name, stdout, stderr);
    }

    // Read the generated Rust file
    let rust_file = wj_file.replace(".wj", ".rs");
    let rust_path = PathBuf::from("./build").join(&rust_file);
    let generated_rust = fs::read_to_string(&rust_path)
        .unwrap_or_else(|_| panic!("Failed to read generated Rust file: {:?}", rust_path));

    // Check that all expected imports are present
    for expected_import in expected_imports {
        if !generated_rust.contains(expected_import) {
            panic!(
                "✗ {} failed: Expected import '{}' not found in generated Rust:\n{}",
                test_name, expected_import, generated_rust
            );
        }
    }

    println!("✓ {} passed", test_name);
}

fn compile_should_fail(code: &str, expected_error: &str, test_name: &str) {
    match compile_wj_code(code) {
        Ok(_) => panic!("✗ {} should have failed but succeeded", test_name),
        Err(e) => {
            if e.contains(expected_error) {
                println!("✓ {} passed (correctly rejected)", test_name);
            } else {
                panic!(
                    "✗ {} failed with wrong error.\nExpected: {}\nGot: {}",
                    test_name, expected_error, e
                );
            }
        }
    }
}

// ============================================================================
// TEST 1: Tuple Enum Variants - Definition
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    x + y
}
"#;
    compile_should_succeed(code, "let_tuple_destructuring");
}

#[test]
fn test_let_nested_tuple_destructuring() {
    let code = r#"
fn test() -> i32 {
    let ((a, b), (c, d)) = ((1, 2), (3, 4))
    a + b + c + d
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
    compile_should_fail(
        code,
        "Use '::' for module paths",
        "module_path_slash_rejected",
    );
}

#[test]
fn test_module_path_dot_rejected() {
    let code = r#"
use std.fs
"#;
    compile_should_fail(
        code,
        "Use '::' for module paths",
        "module_path_dot_rejected",
    );
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
// ============================================================================
// STRUCT PATTERN TESTS
// ============================================================================

#[test]
fn test_struct_pattern_basic() {
    let code = r#"
enum Shape {
    Circle { radius: f32 },
    Rectangle { width: f32, height: f32 },
}

fn calculate_area(shape: Shape) -> f32 {
    match shape {
        Shape::Circle { radius: r } => {
            return 3.14159 * r * r
        }
        Shape::Rectangle { width: w, height: h } => {
            return w * h
        }
    }
}
"#;
    compile_should_succeed(code, "struct_pattern_basic");
}

#[test]
fn test_struct_pattern_with_wildcard() {
    let code = r#"
enum Shape {
    Rectangle { width: f32, height: f32 },
}

fn has_large_width(shape: Shape) -> bool {
    match shape {
        Shape::Rectangle { width: w, height: _ } => w > 10.0,
    }
}
"#;
    compile_should_succeed(code, "struct_pattern_with_wildcard");
}

#[test]
fn test_struct_pattern_multiple_variants() {
    let code = r#"
enum Shape {
    Circle { radius: f32 },
    Rectangle { width: f32, height: f32 },
    Triangle { base: f32, height: f32 },
}

fn get_first_dimension(shape: Shape) -> f32 {
    match shape {
        Shape::Circle { radius: r } => r,
        Shape::Rectangle { width: w, height: _ } => w,
        Shape::Triangle { base: b, height: _ } => b,
    }
}
"#;
    compile_should_succeed(code, "struct_pattern_multiple_variants");
}

#[test]
fn test_module_import_resolution() {
    // This test verifies that when multiple types are imported from the same module,
    // the compiler correctly resolves which module file each type is defined in.
    // Bug: Compiler was generating "use super::collider2d::Collider2D" when it should
    // generate "use super::module_import_resolution::Collider2D" because Collider2D
    // is defined in module_import_resolution.wj, not in a separate collider2d.wj file.

    compile_and_check_generated_rust(
        "module_import_resolution_user.wj",
        &[
            "use module_import_resolution::RigidBody2D",
            "use module_import_resolution::Collider2D", // NOT collider2d!
        ],
        "module_import_resolution",
    );
}

#[test]
fn test_operator_precedence_negation() {
    // Test that !(a || b) generates correct Rust with parentheses preserved
    // Bug: Compiler was generating !a || b instead of !(a || b)

    compile_and_check_generated_rust(
        "operator_precedence.wj",
        &[
            "!(a || b)", // Must have parentheses around the OR
            "!(a && b)", // Must have parentheses around the AND
        ],
        "operator_precedence",
    );
}

#[test]
fn test_array_indexing_with_int() {
    // Test that array indexing with 'int' type automatically casts to usize
    // Bug: arr[index] where index: int generates arr[index as i64] which fails
    // Expected: arr[index as usize] or automatic conversion

    compile_and_check_rust_compiles("array_indexing.wj", "array_indexing_with_int");
}

#[test]
fn test_param_mutability_inference() {
    // Test that function parameters are automatically inferred as &mut when mutated
    // Bug: fn move_point(p: Point, dx: f32) { p.x = ... } generates p: Point instead of p: &mut Point
    // Expected: Automatic inference of &mut for mutated parameters

    compile_and_check_rust_compiles(
        "param_mutability_inference.wj",
        "param_mutability_inference",
    );
}

#[test]
#[ignore] // TODO: Requires windjammer_runtime crate in test environment
fn test_trait_impl_stdlib() {
    // Test that trait implementations match trait signatures exactly
    // Bug: fn add(self, other: Point) was generating other: &Point
    // Expected: Trait method parameters should NOT be inferred, use trait signature

    compile_and_check_rust_compiles("trait_impl_stdlib.wj", "trait_impl_stdlib");
}

#[test]
#[ignore] // TODO: Requires windjammer_runtime crate in test environment
fn test_copy_type_ownership() {
    // Test that Copy types used in operator expressions remain owned
    // Bug: fn distance(a: Vec2, b: Vec2) with a - b generates a: &Vec2, b: &Vec2
    // This breaks operator overloading because Sub is not implemented for &Vec2
    // Expected: Copy types should remain owned for operator compatibility

    compile_and_check_rust_compiles("copy_type_ownership.wj", "copy_type_ownership");
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
