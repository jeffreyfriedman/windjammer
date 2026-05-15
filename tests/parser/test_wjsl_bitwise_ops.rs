//! TDD tests for WJSL bitwise operators
//!
//! Bug: "Unexpected token in expression: BitAnd" when parsing a & b or &expr (address-of)

use windjammer::wjsl::transpile_wjsl;

#[test]
fn test_bitwise_and_in_expression() {
    let source = r#"
fn mask(a: u32, b: u32) -> u32 {
    return a & b;
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("a & b"));
}

#[test]
fn test_address_of_for_atomic() {
    let source = r#"
@group(0) @binding(0) storage read_write draw_count: atomic<u32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let base_idx = atomicAdd(&draw_count, 1u);
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("atomicAdd(&draw_count"));
}

#[test]
fn test_bitwise_or_and_shift() {
    let source = r#"
fn test_ops(a: u32, b: u32) -> u32 {
    return (a | b) >> 1u;
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("a | b"));
    assert!(wgsl.contains(">>"));
}
