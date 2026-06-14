/// TDD test: Assignment statements should be parsed in blocks
///
/// Bug: parse_and_check() skips over assignment statements like `x = value;`
/// inside blocks, leading to "Unexpected token" errors when the expression
/// parser encounters the `=` token.


#[test]
fn test_assignment_in_if_block() {
    let source = r#"
        @compute @workgroup_size(8, 8)
        fn main() {
            let mut result = vec3(0.0);
            if (true) {
                result = vec3(1.0, 2.0, 3.0);
            }
        }
    "#;
    
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(result.is_ok(), "Should parse assignment in if block: {:?}", result.err());
}

#[test]
fn test_multiple_assignments_in_blocks() {
    let source = r#"
        @compute @workgroup_size(8, 8)
        fn main() {
            let mut x = 0.0;
            let mut y = 0.0;
            if (true) {
                x = 1.0;
                y = 2.0;
            }
        }
    "#;
    
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(result.is_ok(), "Should parse multiple assignments: {:?}", result.err());
}

#[test]
fn test_assignment_with_expression() {
    let source = r#"
        @compute @workgroup_size(8, 8)
        fn main() {
            let mut face_n = vec3(0.0);
            let step_sign = vec3(-1.0, 1.0, 0.0);
            if (true) {
                face_n = vec3(-step_sign.x, 0.0, 0.0);
            }
        }
    "#;
    
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(result.is_ok(), "Should parse assignment with swizzle expression: {:?}", result.err());
}
