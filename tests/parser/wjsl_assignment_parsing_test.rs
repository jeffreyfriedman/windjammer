// TDD: Minimal test for assignment parsing bug
use anyhow::Result;

#[test]
fn test_simple_assignment() -> Result<()> {
    let src = r#"
        fn test() {
            let mut x = 0;
            x = 5;
        }
    "#;
    
    let result = windjammer::wjsl::transpile_wjsl_to_wgsl(src, "test.wjsl");
    if let Err(e) = &result {
        eprintln!("ERROR: {}", e);
    }
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_field_assignment() -> Result<()> {
    let src = r#"
        struct Vec2 { x: f32, y: f32 }
        
        fn test() {
            var v: Vec2;
            v.x = 1.0;
        }
    "#;
    
    let result = windjammer::wjsl::transpile_wjsl_to_wgsl(src, "test.wjsl");
    if let Err(e) = &result {
        eprintln!("ERROR: {}", e);
    }
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_compound_assignment() -> Result<()> {
    let src = r#"
        fn test() {
            var x = 0.0;
            x += 1.0;
        }
    "#;
    
    let result = windjammer::wjsl::transpile_wjsl_to_wgsl(src, "test.wjsl");
    if let Err(e) = &result {
        eprintln!("ERROR: {}", e);
    }
    assert!(result.is_ok());
    Ok(())
}
