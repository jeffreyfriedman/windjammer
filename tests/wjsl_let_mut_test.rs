#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

/// TDD: `let mut` support in WJSL transpiler
///
/// Bug: WJSL should follow Windjammer's philosophy and use `let mut` for mutable
/// variables (not `var`). The transpiler should convert `let mut` → `var` when
/// generating WGSL output.
///
/// Issue: voxel_raymarch.wjsl uses `let mut` syntax but fails with "Unknown identifier"
/// during type checking.
fn transpile(source: &str) -> Result<String, String> {
    windjammer::wjsl::transpile_wjsl(source).map_err(|e| e.to_string())
}

#[test]
fn test_let_mut_simple() {
    let source = r#"
@fragment
fn main() {
    let mut x = 1.0;
}
"#;

    let result = transpile(source);
    assert!(result.is_ok(), "Should transpile 'let mut' successfully: {:?}", result.err());
    
    let wgsl = result.unwrap();
    assert!(wgsl.contains("var x"), "Should convert 'let mut' to 'var': {}", wgsl);
}

#[test]
fn test_let_mut_with_usage() {
    let source = r#"
@fragment
fn main() {
    let mut x = 1.0;
    x = x + 2.0;
}
"#;

    let result = transpile(source);
    assert!(result.is_ok(), "Should transpile 'let mut' with usage: {:?}", result.err());
    
    let wgsl = result.unwrap();
    assert!(wgsl.contains("var x"), "Should convert 'let mut' to 'var': {}", wgsl);
    assert!(wgsl.contains("x = x + 2.0"), "Should preserve usage: {}", wgsl);
}

#[test]
fn test_let_mut_voxel_pattern() {
    // This is the exact pattern from voxel_raymarch.wjsl line 206
    let source = r#"
@fragment
fn main() {
    let pos = vec3(0.0);
    let inv_vs = 1.0;
    let vs = 1.0;
    
    let mut voxel = floor(pos * inv_vs) * vs;
    voxel = voxel + vec3(1.0);
}
"#;

    let result = transpile(source);
    assert!(result.is_ok(), "Should transpile voxel pattern: {:?}", result.err());
    
    let wgsl = result.unwrap();
    assert!(wgsl.contains("var voxel"), "Should convert 'let mut voxel' to 'var voxel': {}", wgsl);
}

#[test]
fn test_let_mut_with_type_annotation() {
    let source = r#"
@fragment
fn main() {
    let mut x: f32 = 1.0;
    x = 2.0;
}
"#;

    let result = transpile(source);
    assert!(result.is_ok(), "Should transpile 'let mut' with type: {:?}", result.err());
    
    let wgsl = result.unwrap();
    assert!(wgsl.contains("var x: f32"), "Should preserve type annotation: {}", wgsl);
}
