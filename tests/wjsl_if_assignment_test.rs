/// TDD test: Assignment in if statement (single-line body)
///
/// Bug: `if (cond) { x = value; }` fails with "Unexpected token: Assign"

use windjammer::wjsl::transpile_wjsl;

#[test]
fn test_assignment_in_single_line_if() {
    let source = r#"
        @compute @workgroup_size(8, 8)
        fn main() {
            let mut x = 0.0;
            if (true) { x = 1.0; }
        }
    "#;
    
    let result = transpile_wjsl(source);
    assert!(result.is_ok(), "Should parse assignment in single-line if: {:?}", result.err());
}

#[test]
fn test_assignment_in_nested_if() {
    let source = r#"
        @compute @workgroup_size(8, 8)
        fn main() {
            let mut face_n = vec3(0.0);
            let step_sign = vec3(-1.0, 1.0, 0.0);
            if (true) { face_n = vec3(-step_sign.x, 0.0, 0.0); }
        }
    "#;
    
    let result = transpile_wjsl(source);
    assert!(result.is_ok(), "Should parse field access in assignment: {:?}", result.err());
}
