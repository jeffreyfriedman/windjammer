//! TDD: heat_distortion.wjsl transpiles to valid WGSL (screen-space heat shimmer).
//! Tests the heat distortion compute shader which produces realistic heat haze
//! near fire sources, explosions, and hot surfaces.

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
fn test_wjsl_heat_distortion_transpiles() {
    let wgsl = transpile_shader("heat_distortion.wjsl");

    assert!(!wgsl.is_empty(), "transpiled WGSL must not be empty");
    assert!(
        wgsl.contains("@compute"),
        "output should declare @compute entry point"
    );
    assert!(
        wgsl.contains("HeatParams"),
        "output should define HeatParams struct"
    );
    assert!(
        wgsl.contains("workgroup_size"),
        "output should specify workgroup_size"
    );
}

#[test]
fn test_heat_distortion_binding_layout() {
    let wgsl = transpile_shader("heat_distortion.wjsl");

    assert!(
        wgsl.contains("@binding(0)") && wgsl.contains("var<uniform> heat_params"),
        "binding 0 must be HeatParams uniform\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("@binding(1)") && wgsl.contains("camera"),
        "binding 1 must be CameraUniforms uniform\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("@binding(2)") && wgsl.contains("color_input"),
        "binding 2 must be color_input storage read\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("@binding(3)") && wgsl.contains("color_output"),
        "binding 3 must be color_output storage read_write\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_heat_distortion_uv_offset() {
    let wgsl = transpile_shader("heat_distortion.wjsl");

    assert!(
        wgsl.contains("offset") || wgsl.contains("distort"),
        "heat distortion must compute UV offset for shimmer\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("sin("),
        "heat distortion must use sin() for wave-based UV perturbation\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("time"),
        "heat distortion must animate over time\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_heat_distortion_heat_source() {
    let wgsl = transpile_shader("heat_distortion.wjsl");

    assert!(
        wgsl.contains("heat_source") || wgsl.contains("source_pos"),
        "must define heat source position\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("intensity") || wgsl.contains("strength"),
        "must have heat intensity parameter\nWGSL:\n{}",
        wgsl
    );

    assert!(
        wgsl.contains("radius") || wgsl.contains("falloff"),
        "must have heat falloff/radius\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_heat_distortion_workgroup_size() {
    let wgsl = transpile_shader("heat_distortion.wjsl");

    assert!(
        wgsl.contains("@workgroup_size(8, 8, 1)"),
        "heat distortion must use 8x8 workgroup to match pipeline dispatch\nWGSL:\n{}",
        wgsl
    );
}
