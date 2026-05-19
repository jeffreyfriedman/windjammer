/// Test control flow scoping: variables from outer scope should be accessible in inner scopes
use windjammer::wjsl::transpile;

#[test]
fn test_outer_var_in_for_loop() {
    let source = r#"
@fragment
fn main() {
    let mut voxel = vec3(0.0);
    
    for (var i = 0u; i < 10u; i++) {
        if (any(voxel < vec3(0.0))) {
            break;
        }
    }
}
"#;
    let result = transpile(source);
    assert!(
        result.is_ok(),
        "Outer variable should be accessible in nested for+if: {:?}",
        result.err()
    );
}

#[test]
fn test_for_loop_var_in_body() {
    let source = r#"
@fragment
fn main() {
    for (var depth = 0u; depth < 10u; depth++) {
        if (depth > 5u) {
            break;
        }
    }
}
"#;
    let result = transpile(source);
    assert!(
        result.is_ok(),
        "For loop variable should be accessible in loop body: {:?}",
        result.err()
    );
}

#[test]
fn test_nested_scopes() {
    let source = r#"
@fragment
fn main() {
    let mut outer = 1.0;
    
    for (var i = 0u; i < 10u; i++) {
        let inner = 2.0;
        
        if (outer > inner) {
            let nested = 3.0;
            outer = outer + nested;
        }
    }
}
"#;
    let result = transpile(source);
    assert!(
        result.is_ok(),
        "Nested scopes should work correctly: {:?}",
        result.err()
    );
}
