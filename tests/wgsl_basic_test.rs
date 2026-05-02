/// TDD test: Basic WGSL compilation
///
/// Tests that the WGSL backend can compile a simple function to WGSL.
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_simple_add_function() {
    let source = r#"
pub fn add(x: uint, y: uint) -> uint {
    x + y
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated WGSL:\n{}", generated);

    // Check that the function was generated
    assert!(
        generated.contains("fn add"),
        "Generated WGSL should contain 'fn add'. Got:\n{}",
        generated
    );

    // Check parameters
    assert!(
        generated.contains("x: u32"),
        "Should have u32 parameter. Got:\n{}",
        generated
    );

    // Check return type
    assert!(
        generated.contains("-> u32"),
        "Should have u32 return type. Got:\n{}",
        generated
    );

    // Check function body
    assert!(
        generated.contains("return"),
        "Should have return statement. Got:\n{}",
        generated
    );
}

#[test]
fn test_primitive_types() {
    let source = r#"
pub fn test_types(a: uint, b: int32, c: float, d: bool) -> float {
    c
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated WGSL:\n{}", generated);

    // Check type mappings
    assert!(generated.contains("a: u32"));
    assert!(generated.contains("b: i32"));
    assert!(generated.contains("c: f32"));
    assert!(generated.contains("d: bool"));
    assert!(generated.contains("-> f32"));
}

#[test]
fn test_binary_operations() {
    let source = r#"
pub fn test_ops(x: uint, y: uint) -> uint {
    let sum = x + y
    let diff = x - y
    let prod = x * y
    let quot = x / y
    sum
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated WGSL:\n{}", generated);

    // Check operations are generated
    assert!(generated.contains("+"));
    assert!(generated.contains("-"));
    assert!(generated.contains("*"));
    assert!(generated.contains("/"));

    // Check let statements
    assert!(generated.contains("let sum"));
    assert!(generated.contains("let diff"));
    assert!(generated.contains("let prod"));
    assert!(generated.contains("let quot"));
}

#[test]
fn test_if_statement() {
    let source = r#"
pub fn max(x: uint, y: uint) -> uint {
    if x > y {
        x
    } else {
        y
    }
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated WGSL:\n{}", generated);

    // Check if/else structure
    assert!(generated.contains("if"));
    assert!(generated.contains("else"));
    assert!(generated.contains(">"));
}

#[test]
fn test_while_loop() {
    let source = r#"
pub fn count(n: uint) -> uint {
    let mut i = 0
    while i < n {
        i = i + 1
    }
    i
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated WGSL:\n{}", generated);

    // Check while loop
    assert!(generated.contains("while"));
    assert!(generated.contains("<"));
}
