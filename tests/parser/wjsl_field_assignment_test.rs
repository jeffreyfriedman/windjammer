// Test: Simple field assignment
// This should compile without errors

use anyhow::Result;

#[test]
fn test_wjsl_field_assignment() -> Result<()> {
    let src = r#"
        struct Output {
            position: vec4<f32>,
        }
        
        fn main() -> Output {
            var result: Output;
            result.position = vec4(0.0, 0.0, 0.0, 1.0);
            return result;
        }
    "#;

    // Try to parse and type-check this shader
    let result = windjammer::wjsl::transpile_wjsl_to_wgsl(src, "test.wjsl");
    
    if let Err(e) = &result {
        eprintln!("Transpilation error: {}", e);
    }
    
    assert!(result.is_ok(), "Failed to transpile: {:?}", result.err());
    Ok(())
}
