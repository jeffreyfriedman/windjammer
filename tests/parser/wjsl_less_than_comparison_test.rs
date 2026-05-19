/// Test that < comparison operator doesn't get confused with generic type parameters
use windjammer::wjsl::transpile_wjsl;

#[test]
fn test_less_than_not_confused_with_generic() {
    let source = r#"
@fragment
fn main() {
    let x = 5.0;
    let y = x < 6.0;
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "Less-than comparison should not be confused with generic: {:?}",
        result.err()
    );
}

#[test]
fn test_less_than_before_vec_constructor() {
    let source = r#"
@fragment
fn main() {
    let t_entry = 5.0;
    let use_gradient = t_entry < 6.0;
    let mut v = vec3(1.0, 2.0, 3.0);
}
"#;
    let result = transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "Less-than should work before vec constructor: {:?}",
        result.err()
    );
}
