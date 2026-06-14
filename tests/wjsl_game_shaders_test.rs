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

//! TDD tests for WJSL game shader transpilation.
//!
//! Validates that game shaders written in WJSL can be correctly transpiled
//! to WGSL by the transpile_wjsl() pipeline.

use std::path::Path;
use windjammer::wjsl::transpile_wjsl;

fn game_shaders_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammer-game/windjammer-game-core/src/shaders")
}

fn game_shaders_available() -> bool {
    game_shaders_dir().exists()
}

fn transpile_shader(filename: &str) -> String {
    let shader_dir = game_shaders_dir();
    let source = std::fs::read_to_string(shader_dir.join(filename))
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));
    windjammer::wjsl::transpile_wjsl_with_includes(&source, &shader_dir)
        .unwrap_or_else(|e| panic!("{} should transpile: {}", filename, e))
}

fn read_shader_source(filename: &str) -> String {
    let shader_dir = game_shaders_dir();
    let source = std::fs::read_to_string(shader_dir.join(filename))
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));
    windjammer::wjsl::resolve_includes(&source, &shader_dir, &mut Vec::new())
        .unwrap_or_else(|e| panic!("Failed to resolve includes for {}: {}", filename, e))
}

// =============================================================================
// Unit tests for individual WJSL features
// =============================================================================

#[test]
fn test_wjsl_scalar_cast_f32() {
    let source = r#"
@group(0) @binding(0) uniform x: u32;
@group(0) @binding(1) storage read_write out: array<f32>;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let val = f32(x);
    out[0u] = val;
}
"#;
    let wgsl = transpile_wjsl(source).expect("scalar f32() cast should transpile");
    assert!(wgsl.contains("f32("), "Output should contain f32 cast");
}

#[test]
fn test_wjsl_scalar_cast_u32() {
    let source = r#"
@group(0) @binding(0) uniform x: f32;
@group(0) @binding(1) storage read_write out: array<u32>;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let val = u32(x);
    out[0u] = val;
}
"#;
    transpile_wjsl(source).expect("scalar u32() cast should transpile");
}

#[test]
fn test_wjsl_parameterized_vec_constructor() {
    let source = r#"
@group(0) @binding(0) storage read_write out: array<vec4<f32> >;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let p = vec2<f32>(1.0, 2.0);
    let q = vec3<f32>(1.0, 2.0, 3.0);
    let r = vec4<f32>(1.0, 2.0, 3.0, 4.0);
    out[0u] = r;
}
"#;
    transpile_wjsl(source).expect("parameterized vec constructor should transpile");
}

#[test]
fn test_wjsl_var_declaration_tracked() {
    let source = r#"
@group(0) @binding(0) uniform seed: u32;
@group(0) @binding(1) storage read_write out: array<u32>;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    var s = seed;
    return;
}
"#;
    transpile_wjsl(source).expect("var declaration should be tracked in symbols");
}

#[test]
fn test_wjsl_var_with_type_annotation() {
    let source = r#"
@group(0) @binding(0) storage read_write out: array<f32>;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    var x: f32 = 0.0;
    return;
}
"#;
    transpile_wjsl(source).expect("var with type annotation should transpile");
}

// =============================================================================
// Full game shader integration tests
// =============================================================================

#[test]
fn test_wjsl_solid_color_transpiles() {
    let source = r#"
@group(0) @binding(0) uniform screen_size: vec2<u32>;
@group(0) @binding(1) storage read_write output: array<vec4<f32> >;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= screen_size.x || id.y >= screen_size.y) { return; }
    let pixel_idx = id.y * screen_size.x + id.x;
    output[pixel_idx] = vec4(1.0, 0.0, 1.0, 1.0);
}
"#;
    let wgsl = transpile_wjsl(source).expect("solid_color WJSL should transpile");
    assert!(wgsl.contains("@compute"), "Should have @compute in output");
    assert!(wgsl.contains("vec4"), "Should have vec4 in output");
}

#[test]
fn test_wjsl_voxel_composite_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("voxel_composite.wjsl");
    assert!(!wgsl.is_empty(), "Generated WGSL should not be empty");
}

#[test]
fn test_wjsl_voxel_lighting_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("voxel_lighting.wjsl");
    assert!(!wgsl.is_empty());
}

#[test]
fn test_wjsl_voxel_denoise_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let source = std::fs::read_to_string(
        "../windjammer-game/windjammer-game-core/shaders/voxel_denoise.wjsl",
    );
    if let Ok(source) = source {
        match transpile_wjsl(&source) {
            Ok(wgsl) => {
                assert!(!wgsl.is_empty());
            }
            Err(e) => {
                panic!("voxel_denoise.wjsl failed to transpile: {}", e);
            }
        }
    }
}

#[test]
fn test_wjsl_voxel_raymarch_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("voxel_raymarch.wjsl");
    assert!(!wgsl.is_empty());
}

#[test]
fn test_wjsl_explicit_u32_to_f32_cast_in_vec_constructor() {
    let source = r#"
@group(0) @binding(0) storage read_write out: array<vec4<f32> >;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel = vec2<f32>(f32(id.x), f32(id.y));
    out[0u] = vec4<f32>(pixel.x, pixel.y, 0.0, 1.0);
}
"#;
    let wgsl = transpile_wjsl(source)
        .expect("explicit f32() casts in vec2<f32> constructor should transpile");
    assert!(wgsl.contains("f32("), "output should retain f32() casts");
    assert!(
        wgsl.contains("vec2<f32>"),
        "output should have vec2<f32> constructor"
    );
}

#[test]
fn test_wjsl_u32_to_u32_cast_for_screen_bounds() {
    let source = r#"
struct Camera {
    screen_size: vec2<f32>,
}
@group(0) @binding(0) uniform camera: Camera;
@group(0) @binding(1) storage read_write out: array<u32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let w = u32(camera.screen_size.x);
    let h = u32(camera.screen_size.y);
    if (id.x >= w || id.y >= h) { return; }
    let idx = id.y * w + id.x;
    out[idx] = 1u;
}
"#;
    let wgsl = transpile_wjsl(source).expect("u32() casts from f32 screen_size should transpile");
    assert!(wgsl.contains("u32("), "output should contain u32() casts");
}

#[test]
fn test_wjsl_i32_to_u32_cast_in_index() {
    let source = r#"
@group(0) @binding(0) storage read_write buf: array<f32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let sx = i32(id.x) + 1i;
    let idx = u32(sx);
    buf[idx] = 1.0;
}
"#;
    transpile_wjsl(source).expect("i32 to u32 cast should transpile cleanly");
}

#[test]
fn test_wjsl_function_call_postfix_swizzle() {
    let source = r#"
struct MaterialData {
    color: vec4<f32>,
    roughness: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

@group(0) @binding(0) storage read materials: array<f32 >;
@group(0) @binding(1) storage read_write color_output: array<vec4<f32> >;

fn get_material_color(material_id: u32) -> vec4<f32> {
    let base = material_id * 16u;
    return vec4(
        materials[base + 0u],
        materials[base + 1u],
        materials[base + 2u],
        materials[base + 3u]
    );
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let albedo = get_material_color(1u).rgb;
    color_output[0u] = vec4(albedo, 1.0);
}
"#;
    let wgsl =
        transpile_wjsl(source).expect("function call followed by .rgb swizzle should transpile");
    assert!(wgsl.contains(".rgb"), "output should preserve .rgb swizzle");
}

#[test]
fn test_wjsl_nested_struct_member_access() {
    let source = r#"
struct LightingParams {
    sun_direction: vec3<f32>,
    _pad1: f32,
    sun_color: vec3<f32>,
    sun_intensity: f32,
    sky_color: vec3<f32>,
    _pad2: f32,
    ambient_intensity: f32,
}

@group(0) @binding(0) uniform lighting: LightingParams;
@group(0) @binding(1) storage read_write out: array<vec4<f32> >;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let sky = vec3(lighting.sky_color.x, lighting.sky_color.y, lighting.sky_color.z);
    let sun_dir = normalize(-lighting.sun_direction);
    out[0u] = vec4(sky * lighting.ambient_intensity, 1.0);
}
"#;
    let wgsl = transpile_wjsl(source).expect("nested struct.member.swizzle should transpile");
    assert!(
        wgsl.contains("lighting.sky_color"),
        "should access struct member"
    );
}

#[test]
fn test_wjsl_full_lighting_shader() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("voxel_lighting.wjsl");
    assert!(
        wgsl.contains("fn main"),
        "transpiled output should contain main function"
    );
}

#[test]
fn test_wjsl_gbuffer_struct_consistency() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let raymarch = read_shader_source("voxel_raymarch.wjsl");
    let lighting = read_shader_source("voxel_lighting.wjsl");
    let denoise = read_shader_source("voxel_denoise.wjsl");

    fn extract_gbuffer_struct(source: &str) -> String {
        let start = source
            .find("struct GBufferPixel")
            .expect("GBufferPixel not found");
        let end = source[start..].find('}').expect("closing brace not found") + start + 1;
        source[start..end]
            .lines()
            .map(|l| {
                let l = l.trim();
                if let Some(idx) = l.find("//") {
                    l[..idx].trim()
                } else {
                    l
                }
            })
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    let rm_struct = extract_gbuffer_struct(&raymarch);
    let lt_struct = extract_gbuffer_struct(&lighting);
    let dn_struct = extract_gbuffer_struct(&denoise);

    assert_eq!(
        rm_struct, lt_struct,
        "Raymarch and Lighting GBufferPixel must match"
    );
    assert_eq!(
        rm_struct, dn_struct,
        "Raymarch and Denoise GBufferPixel must match"
    );
}

/// TDD: Every top-level game shader in src/shaders/ must transpile (roadmap: 27+ passes).
#[test]
fn test_all_top_level_game_shaders_transpile() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let dir = game_shaders_dir();
    let mut count = 0usize;
    for entry in std::fs::read_dir(&dir).expect("read src/shaders") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("wjsl") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("shader filename");
        transpile_shader(name);
        count += 1;
    }
    assert!(
        count >= 27,
        "expected at least 27 top-level .wjsl shaders, found {}",
        count
    );
}
