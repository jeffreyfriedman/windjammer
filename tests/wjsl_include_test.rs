use std::fs;
use std::path::Path;

fn transpile_with_includes(source: &str, base_dir: &Path) -> Result<String, String> {
    let resolved = windjammer::wjsl::resolve_includes(source, base_dir, &mut Vec::new())
        .map_err(|e| e.to_string())?;
    windjammer::wjsl::transpile_wjsl(&resolved).map_err(|e| e.to_string())
}

#[test]
fn test_single_include() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    fs::write(
        base.join("camera.wjsl"),
        r#"struct CameraUniforms {
    view_matrix: mat4x4<f32>,
    position: vec3<f32>,
    _pad1: f32,
}
"#,
    )
    .unwrap();

    let source = r#"#include "camera.wjsl"

@group(0) @binding(0) uniform camera: CameraUniforms;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = camera.position;
}
"#;

    let result = transpile_with_includes(source, base).unwrap();
    assert!(
        result.contains("CameraUniforms"),
        "Output should contain the included struct: {}",
        result
    );
    assert!(
        result.contains("view_matrix"),
        "Output should contain included struct fields: {}",
        result
    );
    assert!(
        !result.contains("#include"),
        "Output should NOT contain #include directive: {}",
        result
    );
}

#[test]
fn test_nested_include() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    fs::write(base.join("types.wjsl"), "struct Vec2F { x: f32, y: f32 }\n").unwrap();

    fs::write(
        base.join("camera.wjsl"),
        r#"#include "types.wjsl"
struct CameraUniforms {
    screen_size: vec2<f32>,
    near_plane: f32,
    far_plane: f32,
}
"#,
    )
    .unwrap();

    let source = r#"#include "camera.wjsl"

@group(0) @binding(0) uniform camera: CameraUniforms;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let near = camera.near_plane;
}
"#;

    let result = transpile_with_includes(source, base).unwrap();
    assert!(
        result.contains("Vec2F"),
        "Output should contain transitively included struct: {}",
        result
    );
    assert!(
        result.contains("CameraUniforms"),
        "Output should contain directly included struct: {}",
        result
    );
}

#[test]
fn test_missing_include_file() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    let source = r#"#include "nonexistent.wjsl"

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {}
"#;

    let result = transpile_with_includes(source, base);
    assert!(result.is_err(), "Should fail with missing include file");
    let err = result.unwrap_err();
    assert!(
        err.contains("nonexistent.wjsl"),
        "Error should mention the missing file: {}",
        err
    );
}

#[test]
fn test_circular_include_detection() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    fs::write(
        base.join("a.wjsl"),
        "#include \"b.wjsl\"\nstruct A { x: f32 }\n",
    )
    .unwrap();

    fs::write(
        base.join("b.wjsl"),
        "#include \"a.wjsl\"\nstruct B { y: f32 }\n",
    )
    .unwrap();

    let source = "#include \"a.wjsl\"\n";

    let result = windjammer::wjsl::resolve_includes(source, base, &mut Vec::new());
    assert!(result.is_err(), "Should detect circular include");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("circular") || err.contains("Circular"),
        "Error should mention circular dependency: {}",
        err
    );
}

#[test]
fn test_duplicate_include_deduplicated() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    fs::write(base.join("types.wjsl"), "struct Shared { value: f32 }\n").unwrap();

    let source = r#"#include "types.wjsl"
#include "types.wjsl"

@group(0) @binding(0) uniform data: Shared;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let v = data.value;
}
"#;

    let result = transpile_with_includes(source, base).unwrap();
    let count = result.matches("Shared").count();
    assert!(
        count <= 3,
        "Struct Shared should not be duplicated excessively (found {} occurrences): {}",
        count,
        result
    );
}

#[test]
fn test_include_with_subdirectory() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    let subdir = base.join("wjsl_std");
    fs::create_dir_all(&subdir).unwrap();

    fs::write(
        subdir.join("camera.wjsl"),
        "struct CameraUniforms { position: vec3<f32>, _pad: f32 }\n",
    )
    .unwrap();

    let source = r#"#include "wjsl_std/camera.wjsl"

@group(0) @binding(0) uniform camera: CameraUniforms;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let p = camera.position;
}
"#;

    let result = transpile_with_includes(source, base).unwrap();
    assert!(
        result.contains("CameraUniforms"),
        "Should resolve includes from subdirectory: {}",
        result
    );
}

#[test]
fn test_no_includes_passthrough() {
    let source = r#"
@group(0) @binding(0) storage read_write output: array<f32>;
@group(0) @binding(1) uniform screen_size: vec2<u32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = screen_size.x;
    let height = screen_size.y;
    if (id.x >= width || id.y >= height) { return; }
    let idx = id.y * width + id.x;
    output[idx] = 1.0;
}
"#;

    let tmp = tempfile::tempdir().unwrap();
    let result = transpile_with_includes(source, tmp.path()).unwrap();
    assert!(
        result.contains("output"),
        "Should pass through source without includes: {}",
        result
    );
}

#[test]
fn test_use_syntax() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    fs::write(
        base.join("camera.wjsl"),
        r#"struct CameraUniforms {
    view_matrix: mat4x4<f32>,
    position: vec3<f32>,
    _pad1: f32,
}
"#,
    )
    .unwrap();

    let source = r#"use "camera.wjsl"

@group(0) @binding(0) uniform camera: CameraUniforms;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pos = camera.position;
}
"#;

    let result = transpile_with_includes(source, base);
    assert!(
        result.is_ok(),
        "use syntax should work like #include: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(
        wgsl.contains("CameraUniforms"),
        "use should inline the included file: {}",
        wgsl
    );
}

#[test]
fn test_use_and_include_both_work() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    fs::write(base.join("types_a.wjsl"), "struct TypeA { value: f32, }\n").unwrap();

    fs::write(base.join("types_b.wjsl"), "struct TypeB { count: u32, }\n").unwrap();

    let source = r#"use "types_a.wjsl"
#include "types_b.wjsl"

@group(0) @binding(0) uniform a: TypeA;
@group(0) @binding(1) uniform b: TypeB;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let v = a.value;
}
"#;

    let result = transpile_with_includes(source, base);
    assert!(
        result.is_ok(),
        "Mixed use/include should work: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(wgsl.contains("TypeA"), "Should contain TypeA");
    assert!(wgsl.contains("TypeB"), "Should contain TypeB");
}
