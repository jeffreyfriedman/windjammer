// Test return statement generation
// Dogfooding bug: return statements in else blocks

use windjammer::*;

#[test]
fn test_return_in_else_block() {
    let source = r#"
pub fn check_slot(found: bool) -> bool {
    if found {
        let x = 42
    } else {
        return false
    }
    true
}
"#;

    let result = compile_to_rust(source);
    
    // Should generate "return false" not just "false"
    assert!(result.contains("return false"), 
        "return statement should be preserved in else block, got: {}", result);
    
    // Should not have type mismatch
    assert!(!result.contains("} else {"), 
        "Else block should have return keyword");
}

#[test]
fn test_return_in_if_block() {
    let source = r#"
pub fn early_exit(x: i32) -> bool {
    if x > 10 {
        return true
    }
    false
}
"#;

    let result = compile_to_rust(source);
    
    // Should generate "return true"
    assert!(result.contains("return true"), 
        "return statement should be preserved in if block");
}

#[test]
fn test_return_in_nested_block() {
    let source = r#"
pub fn nested_return(a: bool, b: bool) -> bool {
    if a {
        if b {
            return true
        } else {
            return false
        }
    }
    false
}
"#;

    let result = compile_to_rust(source);
    
    // Both returns should be preserved
    assert!(result.contains("return true"), 
        "return true should be in generated code");
    assert!(result.contains("return false"), 
        "return false should be in generated code");
}

// Helper function to compile a snippet
fn compile_to_rust(source: &str) -> String {
    // This would use the actual compiler
    // For now, placeholder
    use windjammer::compiler::*;
    use windjammer::parser::*;
    
    let ast = parse(source).expect("Parse failed");
    let analyzed = analyze(&ast).expect("Analysis failed");
    codegen_rust(&analyzed).expect("Codegen failed")
}
