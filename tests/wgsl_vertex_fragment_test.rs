// WGSL Vertex + Fragment Shader Tests
// Testing vertex/fragment entry points and struct returns

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn transpile_wj_to_wgsl(source: &str) -> String {
    let tmp = TempDir::new().unwrap();
    let input_file = tmp.path().join("test.wj");
    let output_dir = tmp.path().join("out");
    
    fs::write(&input_file, source).unwrap();
    
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "wj",
            "build",
            input_file.to_str().unwrap(),
            "--target",
            "wgsl",
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to compile");
    
    if !output.status.success() {
        panic!(
            "Compilation failed:\nSTDERR:\n{}\n\nSTDOUT:\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }
    
    let wgsl_file = output_dir.join("test.wgsl");
    fs::read_to_string(&wgsl_file).expect("Should generate WGSL file")
}

#[test]
fn test_vertex_shader_basic() {
    let source = r#"
pub struct VertexOutput {
    position: vec4<float>,
    color: vec4<float>,
}

@vertex
pub fn vs_main(@location(0) pos: vec2<float>) -> VertexOutput {
    VertexOutput {
        position: vec4(pos.x, pos.y, 0.0, 1.0),
        color: vec4(1.0, 1.0, 1.0, 1.0),
    }
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("@vertex"), "Generated:\n{}", generated);
    assert!(generated.contains("vs_main"), "Generated:\n{}", generated);
    assert!(generated.contains("@location(0)"), "Generated:\n{}", generated);
}

#[test]
fn test_fragment_shader_basic() {
    let source = r#"
@fragment
pub fn fs_main(@location(0) color: vec4<float>) -> @location(0) vec4<float> {
    color
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("@fragment"), "Generated:\n{}", generated);
    assert!(generated.contains("fs_main"), "Generated:\n{}", generated);
    assert!(generated.contains("@location(0)"), "Generated:\n{}", generated);
}

#[test]
fn test_builtin_position() {
    let source = r#"
pub struct VertexOutput {
    clip_position: vec4<float>,
    color: vec4<float>,
}

@vertex
pub fn vs_main() -> VertexOutput {
    VertexOutput {
        clip_position: vec4(0.0, 0.0, 0.0, 1.0),
        color: vec4(1.0, 1.0, 1.0, 1.0),
    }
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("@vertex"), "Generated:\n{}", generated);
    assert!(generated.contains("VertexOutput"), "Generated:\n{}", generated);
}

#[test]
fn test_struct_literal_return() {
    let source = r#"
pub struct Output {
    position: vec4<float>,
    color: vec4<float>,
}

pub fn create_output() -> Output {
    Output {
        position: vec4(0.0, 0.0, 0.0, 1.0),
        color: vec4(1.0, 0.0, 0.0, 1.0),
    }
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("Output("), "Generated:\n{}", generated);
    assert!(generated.contains("vec4(0"), "Generated:\n{}", generated);
}
