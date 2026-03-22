//! TDD tests for WJSL body pass-through - ensures let, for, return keywords are preserved.
//!
//! Bug: VGS_WJSL_CONVERSION_REPORT.md - "body pass-through loses let, for, return keywords"

use windjammer::wjsl::{parse_wjsl, transpile_wjsl};

#[test]
fn test_function_body_preserves_let() {
    let source = r#"
fn helper() -> vec3 {
    let x = 1.0;
    let y = 2.0;
    return vec3(x, y, 0.0);
}
"#;
    // Verify parser extracts body correctly (body preservation fix)
    let ast = parse_wjsl(source).unwrap();
    assert_eq!(ast.functions.len(), 1);
    let body = &ast.functions[0].body;
    assert!(body.contains("let x"), "parser body should contain 'let x', got: {:?} (len={})", body, body.len());
    assert!(body.contains("let y"), "parser body should contain 'let y', got: {:?}", body);
    assert!(body.contains("return"), "parser body should contain 'return', got: {:?}", body);

    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("let x"), "wgsl should preserve 'let x'");
    assert!(wgsl.contains("let y"), "wgsl should preserve 'let y'");
    assert!(wgsl.contains("return"), "wgsl should preserve 'return'");
}

#[test]
fn test_function_body_preserves_for() {
    let source = r#"
fn loop_example() {
    for (var i = 0; i < 6; i++) {
        let x = 1.0;
    }
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("for "), "body should preserve 'for'");
    assert!(wgsl.contains("var i"), "body should preserve 'var i'");
    assert!(wgsl.contains("let x"), "body should preserve 'let x'");
}

#[test]
fn test_entry_point_body_preserves_keywords() {
    let source = r#"
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cluster_id = id.x;
    if (cluster_id >= 1u) {
        return;
    }
    return;
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("let cluster_id"), "entry point body should preserve 'let'");
    assert!(wgsl.contains("return"), "entry point body should preserve 'return'");
}
