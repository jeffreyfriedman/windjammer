//! TDD tests for .wjsl (Windjammer Shader Language)

use windjammer::shader::{compile_shader, parse_shader, ScalarType, Type, TypeChecker};

#[test]
fn test_parse_and_compile_simple_shader() {
    let source = r#"
        shader MyShader {
            uniform screen_size: Vec2<f32>
            storage output: array<Vec4<f32>>
        }
    "#;
    let module = parse_shader(source).unwrap();
    assert_eq!(module.name, "MyShader");
    assert_eq!(module.uniforms.len(), 1);
    assert_eq!(module.uniforms[0].name, "screen_size");

    let wgsl = compile_shader(source).unwrap();
    assert!(wgsl.contains("struct Uniforms"));
    assert!(wgsl.contains("vec2<f32>"));
    assert!(wgsl.contains("array<vec4<f32>>"));
}

#[test]
fn test_compile_rejects_f64() {
    let source = "shader S { uniform x: Vec2<f64> }";
    let result = compile_shader(source);
    assert!(result.is_err());
}

#[test]
fn test_wj_shader_compile_cli() {
    use std::fs;
    use std::process::Command;

    let temp = std::env::temp_dir().join("wjsl_test");
    fs::create_dir_all(&temp).unwrap();
    let input = temp.join("test.wjsl");
    fs::write(
        &input,
        "shader S { uniform screen_size: Vec2<f32> storage out: array<Vec4<f32>> }",
    )
    .unwrap();
    let output = temp.join("test.wgsl");

    let wj = env!("CARGO_BIN_EXE_wj");
    let result = Command::new(wj)
        .arg("shader-compile")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output();

    let result = result.expect("wj binary failed to run");
    assert!(result.status.success(), "stderr: {}", String::from_utf8_lossy(&result.stderr));

    let wgsl = fs::read_to_string(&output).unwrap();
    assert!(wgsl.contains("struct Uniforms"));
    assert!(wgsl.contains("vec2<f32>"));
    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_type_checker_integration() {
    let source = "shader S { uniform x: Vec2<f32> }";
    let module = parse_shader(source).unwrap();
    let checker = TypeChecker::new();

    assert!(checker.check_uniform_match(&module, "x", &Type::Vec2(ScalarType::F32)).is_ok());
    assert!(checker.check_uniform_match(&module, "x", &Type::Vec2(ScalarType::F64)).is_err());
}
