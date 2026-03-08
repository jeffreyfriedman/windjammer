// Tests for WGSL buffer bindings and storage qualifiers
// Following TDD approach as per windjammer-development.mdc

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

// ============================================================================
// UNIFORM BUFFER TESTS
// ============================================================================

#[test]
fn test_uniform_buffer_simple() {
    let source = r#"
pub struct CameraUniforms {
    @group(0) @binding(0)
    view_proj: mat4x4<float>,
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    // WGSL uses @group, @binding on the variable declaration, not the struct field
    // But we can verify the struct is generated correctly
    assert!(generated.contains("struct CameraUniforms"));
    assert!(generated.contains("view_proj: mat4x4<f32>"));
}

#[test]
fn test_storage_buffer_read_write() {
    let source = r#"
pub struct ParticleBuffer {
    @group(0) @binding(1) @storage(read_write)
    particles: array<Particle>,
}

pub struct Particle {
    position: vec3<float>,
    velocity: vec3<float>,
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("struct ParticleBuffer"));
    assert!(generated.contains("struct Particle"));
    assert!(generated.contains("particles: array<Particle>"));
}

#[test]
fn test_storage_buffer_read_only() {
    let source = r#"
pub struct MeshData {
    @group(0) @binding(2) @storage(read)
    vertices: array<vec3<float>>,
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("struct MeshData"));
    assert!(generated.contains("vertices: array<vec3<f32>>"));
}

// ============================================================================
// BIND GROUP TESTS
// ============================================================================

#[test]
fn test_multiple_bindings_same_group() {
    let source = r#"
pub struct Bindings {
    @group(0) @binding(0)
    camera: CameraUniforms,
    
    @group(0) @binding(1)
    lighting: LightData,
}

pub struct CameraUniforms {
    view_proj: mat4x4<float>,
}

pub struct LightData {
    position: vec3<float>,
    color: vec3<float>,
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("struct Bindings"));
    assert!(generated.contains("camera: CameraUniforms"));
    assert!(generated.contains("lighting: LightData"));
}

#[test]
fn test_multiple_bind_groups() {
    let source = r#"
pub struct Resources {
    @group(0) @binding(0)
    per_frame: FrameData,
    
    @group(1) @binding(0)
    per_material: MaterialData,
    
    @group(2) @binding(0)
    per_object: ObjectData,
}

pub struct FrameData {
    time: float,
}

pub struct MaterialData {
    albedo: vec4<float>,
}

pub struct ObjectData {
    transform: mat4x4<float>,
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("struct Resources"));
    assert!(generated.contains("per_frame: FrameData"));
    assert!(generated.contains("per_material: MaterialData"));
    assert!(generated.contains("per_object: ObjectData"));
}

// ============================================================================
// GLOBAL VARIABLE TESTS (for actual bind group variables)
// ============================================================================

#[test]
fn test_global_uniform_var() {
    let source = r#"
pub struct CameraUniforms {
    view_proj: mat4x4<float>,
}

@group(0) @binding(0) @uniform
extern let camera: CameraUniforms;

@compute(workgroup_size = [8, 8, 1])
pub fn main_cs(id: vec3<uint>) {
    let vp = camera.view_proj
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    println!("Generated WGSL:\n{}", generated);
    
    assert!(generated.contains("@group(0)"), "Generated:\n{}", generated);
    assert!(generated.contains("@binding(0)"), "Generated:\n{}", generated);
    assert!(generated.contains("camera: CameraUniforms"), "Generated:\n{}", generated);
}

#[test]
fn test_global_storage_var() {
    let source = r#"
pub struct Particle {
    position: vec3<float>,
}

@group(0) @binding(1) @storage(read_write)
extern let particles: array<Particle>;

@compute(workgroup_size = [64, 1, 1])
pub fn update_particles(@builtin(global_invocation_id) id: vec3<uint>) {
    particles[id.x].position = vec3(0.0, 0.0, 0.0)
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("@group(0)"), "Generated:\n{}", generated);
    assert!(generated.contains("@binding(1)"), "Generated:\n{}", generated);
    assert!(generated.contains("particles: array<Particle>"), "Generated:\n{}", generated);
    assert!(generated.contains("storage, read_write"), "Generated:\n{}", generated);
}

// ============================================================================
// TEXTURE/SAMPLER TESTS
// ============================================================================

#[test]
fn test_texture_binding() {
    let source = r#"
@group(0) @binding(0)
extern let albedo_texture: texture_2d<float>;

@group(0) @binding(1)
extern let albedo_sampler: sampler;

@fragment
pub fn main_fs() -> vec4<float> {
    vec4(1.0, 1.0, 1.0, 1.0)
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("albedo_texture: texture_2d<f32>"), "Generated:\n{}", generated);
    assert!(generated.contains("albedo_sampler: sampler"), "Generated:\n{}", generated);
    assert!(generated.contains("@fragment"), "Generated:\n{}", generated);
    assert!(generated.contains("@group(0)"), "Generated:\n{}", generated);
    assert!(generated.contains("@binding(0)"), "Generated:\n{}", generated);
}
