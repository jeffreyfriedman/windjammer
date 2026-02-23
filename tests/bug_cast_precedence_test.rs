// BUG: Operator precedence with 'as' casts and binary operators
//
// DISCOVERED DURING: Voxel system dogfooding (VoxelColor::to_hex)
//
// PROBLEM:
// Windjammer source: `let x = (value as u32) << 8;`
// Generated Rust:    `let x = value as u32 << 8;`  ← WRONG PRECEDENCE!
//
// Rust interprets this as `value as (u32 << 8)` instead of `(value as u32) << 8`
//
// ROOT CAUSE:
// Codegen drops parentheses when generating Expression::Cast with binary operators
//
// FIX:
// Preserve parentheses around cast expressions when followed by binary operators

use windjammer::compile_to_rust;

#[test]
fn test_cast_with_bitshift_preserves_parentheses() {
    let source = r#"
fn test() {
    let value: u8 = 255;
    let shifted = (value as u32) << 8;
    shifted
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    // Should preserve parentheses around the cast
    assert!(
        rust_code.contains("(value as u32) << 8") || 
        rust_code.contains("(value as u32)<<8"),
        "Cast should be parenthesized before shift operator"
    );
}

#[test]
fn test_cast_with_bitwise_or_preserves_parentheses() {
    let source = r#"
fn test() {
    let a: u8 = 1;
    let b: u8 = 2;
    let result = (a as u32) << 24 | (b as u32) << 16;
    result
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    // Both casts should be parenthesized
    assert!(
        rust_code.contains("(a as u32)"),
        "First cast should be parenthesized"
    );
    assert!(
        rust_code.contains("(b as u32)"),
        "Second cast should be parenthesized"
    );
}

#[test]
fn test_voxel_color_to_hex_pattern() {
    // The EXACT pattern from VoxelColor::to_hex that failed
    let source = r#"
struct VoxelColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl VoxelColor {
    pub fn to_hex(self) -> u32 {
        let r_shifted = (self.r as u32) << 24;
        let g_shifted = (self.g as u32) << 16;
        let b_shifted = (self.b as u32) << 8;
        let a_value = self.a as u32;
        r_shifted | g_shifted | b_shifted | a_value
    }
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    // All casts before shifts must be parenthesized
    assert!(
        rust_code.contains("(self.r as u32)"),
        "r cast should be parenthesized"
    );
    assert!(
        rust_code.contains("(self.g as u32)"),
        "g cast should be parenthesized"
    );
    assert!(
        rust_code.contains("(self.b as u32)"),
        "b cast should be parenthesized"
    );
}

#[test]
fn test_cast_alone_no_parens_needed() {
    // When cast is not followed by binary op, no parens needed
    let source = r#"
fn test() {
    let value: u8 = 255;
    let casted = value as u32;
    casted
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    // This is fine either way, but typically no parens needed for standalone cast
    // Just verify it compiles
    assert!(rust_code.contains("as u32"));
}
