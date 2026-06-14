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

fn transpile(source: &str) -> Result<String, String> {
    windjammer::wjsl::transpile_wjsl(source).map_err(|e| e.to_string())
}

#[test]
fn test_let_mut_used_inside_for_loop() {
    let source = r#"
@fragment
fn main() {
    let mut voxel = vec3(0.0);
    
    for (var i = 0u; i < 10u; i++) {
        if (any(voxel < vec3(0.0))) {
            break;
        }
        voxel = voxel + vec3(1.0);
    }
}
"#;
    let result = transpile(source);
    assert!(result.is_ok(), 
        "let mut variable used inside for loop should transpile: {:?}", 
        result.err());
}

#[test]
fn test_let_mut_multiple_uses_in_loop() {
    let source = r#"
@fragment
fn main() {
    let mut pos = vec3(0.0);
    let mut dir = vec3(1.0);
    
    for (var i = 0u; i < 10u; i++) {
        pos = pos + dir;
        dir = normalize(dir);
    }
}
"#;
    let result = transpile(source);
    assert!(result.is_ok(),
        "Multiple let mut variables in loop should transpile: {:?}",
        result.err());
}
