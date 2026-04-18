//! TDD: volumetric_smoke.wjsl transpiles to valid WGSL (raymarched volumetric smoke).
//! Tests the volumetric smoke compute shader which produces realistic smoke plumes
//! using raymarching through a procedural density field with light scattering.

use std::path::Path;

fn transpile_shader(filename: &str) -> String {
    let shader_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammer-game/windjammer-game-core/shaders");
    let source = std::fs::read_to_string(shader_dir.join(filename))
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));
    windjammer::wjsl::transpile_wjsl_with_includes(&source, &shader_dir)
        .unwrap_or_else(|e| panic!("{} should transpile: {}", filename, e))
}

#[test]
fn test_wjsl_volumetric_smoke_transpiles() {
    let wgsl = transpile_shader("smoke_volumetric.wjsl");

    assert!(!wgsl.is_empty(), "transpiled WGSL must not be empty");
    assert!(
        wgsl.contains("@compute"),
        "output should declare @compute entry point"
    );
    assert!(
        wgsl.contains("SmokeParams"),
        "output should define SmokeParams struct"
    );
    assert!(
        wgsl.contains("CameraUniforms"),
        "output should define CameraUniforms struct"
    );
    assert!(
        wgsl.contains("workgroup_size"),
        "output should specify workgroup_size"
    );
}

#[test]
fn test_volumetric_smoke_binding_layout() {
    let wgsl = transpile_shader("smoke_volumetric.wjsl");

    assert!(
        wgsl.contains("@binding(0)") && wgsl.contains("var<uniform> smoke_params"),
        "binding 0 must be SmokeParams uniform\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("@binding(1)") && wgsl.contains("var<uniform> camera"),
        "binding 1 must be CameraUniforms uniform\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("@binding(2)") && wgsl.contains("gbuffer"),
        "binding 2 must be gbuffer storage read\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("@binding(3)") && wgsl.contains("color_output"),
        "binding 3 must be color_output storage read_write\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_volumetric_smoke_raymarching() {
    let wgsl = transpile_shader("smoke_volumetric.wjsl");

    assert!(
        wgsl.contains("step_size") || wgsl.contains("march") || wgsl.contains("ray"),
        "smoke shader must use raymarching with step size\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("density"),
        "smoke shader must compute density along ray\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("transmittance") || wgsl.contains("absorption"),
        "smoke shader must track light transmittance/absorption\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_volumetric_smoke_noise() {
    let wgsl = transpile_shader("smoke_volumetric.wjsl");

    assert!(
        wgsl.contains("noise") || wgsl.contains("hash") || wgsl.contains("fract("),
        "smoke must use procedural noise for density variation\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("time"),
        "smoke must animate over time\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_volumetric_smoke_light_scattering() {
    let wgsl = transpile_shader("smoke_volumetric.wjsl");

    assert!(
        wgsl.contains("scatter") || wgsl.contains("phase") || wgsl.contains("in_scatter"),
        "smoke must compute light scattering (in-scatter term)\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("sun") || wgsl.contains("light_dir"),
        "smoke must consider light/sun direction for scattering\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_volumetric_smoke_workgroup_size() {
    let wgsl = transpile_shader("smoke_volumetric.wjsl");

    assert!(
        wgsl.contains("@workgroup_size(8, 8, 1)"),
        "smoke must use 8x8 workgroup size to match pipeline dispatch\nWGSL:\n{}",
        wgsl
    );
}
