// TDD Test: Windjammer Compiler Bug - extern fn not being transpiled
//
// Bug: extern fn declarations in .wj files are not appearing in generated .rs files
// Expected: extern fn should be transpiled to Rust's extern "C" { pub fn ... }

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_extern_fn_transpiles_to_rust() {
    let test_wj = r#"
extern fn test_simple_function(x: i32) -> i32
extern fn test_no_return(value: f32)
extern fn test_multiple_params(a: u32, b: u32, c: f32) -> bool
extern fn test_string_param(path: string) -> u32
"#;

    let rust_code = test_utils::compile_single(test_wj);

    assert!(
        rust_code.contains("extern \"C\""),
        "Generated Rust should have extern \"C\" block"
    );

    assert!(
        rust_code.contains("pub fn test_simple_function(x: i32) -> i32"),
        "Should transpile: extern fn test_simple_function(x: i32) -> i32"
    );

    assert!(
        rust_code.contains("pub fn test_no_return(value: f32)"),
        "Should transpile: extern fn test_no_return(value: f32)"
    );

    assert!(
        rust_code.contains("pub fn test_multiple_params(a: u32, b: u32, c: f32) -> bool"),
        "Should transpile: extern fn test_multiple_params(a: u32, b: u32, c: f32) -> bool"
    );

    assert!(
        rust_code.contains("test_string_param") && rust_code.contains("FfiString"),
        "String parameters should become FfiString in generated Rust"
    );
}

#[test]
fn test_extern_fn_in_extern_block() {
    let test_wj = r#"
extern fn func_a(x: i32) -> i32
extern fn func_b(y: f32) -> f32
extern fn func_c()
"#;

    let rust_code = test_utils::compile_single(test_wj);

    let extern_count = rust_code.matches("extern \"C\"").count();
    assert_eq!(
        extern_count, 1,
        "Should have exactly one extern \"C\" block"
    );

    assert!(
        rust_code.contains("pub fn func_a"),
        "func_a should be in extern block"
    );
    assert!(
        rust_code.contains("pub fn func_b"),
        "func_b should be in extern block"
    );
    assert!(
        rust_code.contains("pub fn func_c"),
        "func_c should be in extern block"
    );
}
