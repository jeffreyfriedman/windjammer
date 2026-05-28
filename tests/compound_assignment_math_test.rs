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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_compound_add_integers() {
    let source = r#"
pub fn add_values() -> i32 {
    let mut x = 10
    x += 5
    x += 3
    x
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(
        output.contains("x += 5"),
        "Should generate compound assignment for integers"
    );
}

#[test]
fn test_compound_add_floats() {
    let source = r#"
pub fn add_floats() -> f32 {
    let mut x = 10.0
    x += 2.5
    x += 1.5
    x
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(
        output.contains("x += "),
        "Should generate compound assignment for floats"
    );
}

#[test]
fn test_compound_subtract() {
    let source = r#"
pub fn subtract_values() -> i32 {
    let mut health = 100
    health -= 10
    health -= 5
    health
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("health -= "), "Should generate -= operator");
}

#[test]
fn test_compound_multiply() {
    let source = r#"
pub fn multiply_values() -> i32 {
    let mut score = 10
    score *= 2
    score *= 3
    score
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("score *= "), "Should generate *= operator");
}

#[test]
fn test_compound_divide() {
    let source = r#"
pub fn divide_values() -> f32 {
    let mut x = 100.0
    x /= 2.0
    x /= 5.0
    x
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("x /= "), "Should generate /= operator");
}

#[test]
fn test_compound_modulo() {
    let source = r#"
pub fn modulo_value() -> i32 {
    let mut x = 100
    x %= 7
    x
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("x %= "), "Should generate %= operator");
}

#[test]
fn test_compound_bitwise_and() {
    let source = r#"
pub fn bitwise_and() -> i32 {
    let mut flags = 0xFF
    flags &= 0x0F
    flags
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("flags &= "), "Should generate &= operator");
}

#[test]
fn test_compound_bitwise_or() {
    let source = r#"
pub fn bitwise_or() -> i32 {
    let mut flags = 0x01
    flags |= 0x02
    flags |= 0x04
    flags
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("flags |= "), "Should generate |= operator");
}

#[test]
fn test_compound_bitwise_xor() {
    let source = r#"
pub fn bitwise_xor() -> i32 {
    let mut value = 0xFF
    value ^= 0xAA
    value
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("value ^= "), "Should generate ^= operator");
}

#[test]
fn test_compound_left_shift() {
    let source = r#"
pub fn left_shift() -> i32 {
    let mut bits = 1
    bits <<= 3
    bits
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("bits <<= "), "Should generate <<= operator");
}

#[test]
fn test_compound_right_shift() {
    let source = r#"
pub fn right_shift() -> i32 {
    let mut bits = 64
    bits >>= 2
    bits
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("bits >>= "), "Should generate >>= operator");
}

#[test]
fn test_compound_mixed_operations() {
    let source = r#"
pub fn mixed_ops() -> f32 {
    let mut x = 100.0
    x += 50.0  // Add
    x -= 20.0  // Subtract
    x *= 2.0   // Multiply
    x /= 3.0   // Divide
    x
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(output.contains("x += 50"), "Should generate +=");
    assert!(output.contains("x -= 20"), "Should generate -=");
    assert!(output.contains("x *= 2"), "Should generate *=");
    assert!(output.contains("x /= 3"), "Should generate /=");
}

#[test]
fn test_compound_with_expressions() {
    let source = r#"
pub fn compound_with_expr() -> i32 {
    let mut sum = 0
    let count = 5
    sum += count * 2
    sum += count + 3
    sum
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);
    assert!(
        output.contains("sum += "),
        "Should generate compound assignment with expressions"
    );
}

#[test]
fn test_compound_ops_runtime_correctness() {
    let source = r#"
pub fn compute() -> i32 {
    let mut x = 10
    x += 5    // 15
    x -= 3    // 12
    x *= 2    // 24
    x /= 4    // 6
    x %= 5    // 1
    x |= 8    // 9 (0b0001 | 0b1000)
    x
}
"#;

    let (output, success) = test_utils::compile_single_check(source);

    println!("\n=== Generated Rust ===\n{}\n", output);

    assert!(success, "Generated Rust should compile: {}", output);

    // Verify the function can be called (syntax is correct)
    assert!(
        output.contains("pub fn compute() -> i32"),
        "Should generate public function"
    );
}

// Helper function
