/// TDD: String interpolation is the idiomatic Windjammer way to build strings.
/// format!() is Rust leakage and should emit a deprecation warning.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_string_interpolation_compiles_to_format() {
    let source = r#"
pub fn greet(name: String) -> String {
    "Hello, ${name}!"
}
"#;

    let (generated, _stdout, _stderr) = test_utils::compile_via_cli_full(source);
    assert!(
        !generated.is_empty(),
        "String interpolation should compile successfully"
    );
    assert!(
        generated.contains("format!"),
        "String interpolation should lower to format! in Rust codegen. Got:\n{}",
        generated
    );
}

#[test]
fn test_format_macro_emits_deprecation_warning() {
    let source = r#"
pub fn greet(name: String) -> String {
    format!("Hello, {}", name)
}
"#;

    let (_generated, _stdout, stderr) = test_utils::compile_via_cli_full(source);
    assert!(
        stderr.contains("format!() is Rust syntax"),
        "format!() should emit a deprecation warning. Stderr:\n{}",
        stderr
    );
    assert!(
        stderr.contains("string interpolation"),
        "Warning should suggest string interpolation. Stderr:\n{}",
        stderr
    );
}

#[test]
fn test_interpolation_no_warning() {
    let source = r#"
pub fn greet(name: String) -> String {
    "Hello, ${name}!"
}
"#;

    let (_generated, _stdout, stderr) = test_utils::compile_via_cli_full(source);
    assert!(
        !stderr.contains("format!() is Rust syntax"),
        "String interpolation should NOT emit format deprecation warning. Stderr:\n{}",
        stderr
    );
}

#[test]
fn test_interpolation_with_expression() {
    let source = r#"
pub fn describe(x: i32, y: i32) -> String {
    "Point(${x}, ${y})"
}
"#;

    let (generated, _stdout, _stderr) = test_utils::compile_via_cli_full(source);
    assert!(
        !generated.is_empty(),
        "Interpolation with multiple expressions should compile"
    );
    assert!(
        generated.contains("format!"),
        "Should lower to format! macro"
    );
}

#[test]
fn test_interpolation_with_field_access() {
    let source = r#"
pub struct Player {
    pub name: String,
    pub score: i32,
}

pub fn status(p: Player) -> String {
    "${p.name}: ${p.score} points"
}
"#;

    let (generated, _stdout, _stderr) = test_utils::compile_via_cli_full(source);
    assert!(
        !generated.is_empty(),
        "Interpolation with field access should compile"
    );
}
