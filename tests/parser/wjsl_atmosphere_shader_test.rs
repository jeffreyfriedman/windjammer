//! TDD: atmosphere.wjsl transpiles to valid WGSL (procedural sky + fog pass).
//! Validates both basic transpilation and pipeline-correct bindings.

use std::path::Path;

fn game_shaders_available() -> bool {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammer-game/windjammer-game-core/shaders")
        .exists()
}

fn transpile_shader(filename: &str) -> String {
    let shader_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammer-game/windjammer-game-core/shaders");
    let source = std::fs::read_to_string(shader_dir.join(filename))
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));
    windjammer::wjsl::transpile_wjsl_with_includes(&source, &shader_dir)
        .unwrap_or_else(|e| panic!("{} should transpile: {}", filename, e))
}

#[test]
fn test_wjsl_atmosphere_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("atmosphere.wjsl");

    assert!(!wgsl.is_empty(), "transpiled WGSL must not be empty");
    assert!(
        wgsl.contains("@compute"),
        "output should declare @compute entry point"
    );
    assert!(
        wgsl.contains("AtmosphereParams"),
        "output should define AtmosphereParams struct"
    );
    assert!(
        wgsl.contains("CameraUniforms"),
        "output should define CameraUniforms struct"
    );
    assert!(
        wgsl.contains("generate_ray"),
        "output should contain generate_ray"
    );
    assert!(
        wgsl.contains("compute_sky_color"),
        "output should contain compute_sky_color (procedural sky + starfield path)"
    );
    assert!(
        wgsl.contains("gbuffer_base") || wgsl.contains("gbuffer"),
        "output should reference gbuffer layout helpers or buffer"
    );
    assert!(
        wgsl.contains("color_output"),
        "output should reference color_output buffer"
    );
    assert!(
        wgsl.contains("workgroup_size"),
        "output should specify workgroup_size"
    );
    assert!(
        wgsl.contains("exp("),
        "output should contain exp() for exponential fog"
    );
}

#[test]
fn test_atmosphere_binding_layout_matches_pipeline() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("atmosphere.wjsl");

    // Binding 0: AtmosphereParams uniform
    assert!(
        wgsl.contains("@binding(0)") && wgsl.contains("var<uniform> params"),
        "binding 0 must be AtmosphereParams uniform\nWGSL:\n{}",
        wgsl
    );

    // Binding 1: CameraUniforms uniform
    assert!(
        wgsl.contains("@binding(1)") && wgsl.contains("var<uniform> camera"),
        "binding 1 must be CameraUniforms uniform\nWGSL:\n{}",
        wgsl
    );

    // Binding 2: gbuffer storage read
    assert!(
        wgsl.contains("@binding(2)") && wgsl.contains("gbuffer"),
        "binding 2 must be gbuffer storage read\nWGSL:\n{}",
        wgsl
    );

    // Binding 3: color_output storage read_write
    assert!(
        wgsl.contains("@binding(3)") && wgsl.contains("color_output"),
        "binding 3 must be color_output storage read_write\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_atmosphere_sky_rendering_features() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("atmosphere.wjsl");

    // Sky gradient blending (horizon → zenith)
    assert!(
        wgsl.contains("mix(") && wgsl.contains("sky_horizon_color"),
        "atmosphere must blend sky colors with mix()\nWGSL:\n{}",
        wgsl
    );

    // Sun disc rendering
    assert!(
        wgsl.contains("smoothstep(") && wgsl.contains("sun_size"),
        "atmosphere must render sun disc with smoothstep\nWGSL:\n{}",
        wgsl
    );

    // Exponential distance fog
    assert!(
        wgsl.contains("exp(") && wgsl.contains("fog_density"),
        "atmosphere must apply exponential fog\nWGSL:\n{}",
        wgsl
    );

    // No-geometry detection (sky pixels)
    assert!(
        wgsl.contains("no_geometry") || wgsl.contains("material_id"),
        "atmosphere must detect sky vs geometry pixels\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_atmosphere_workgroup_size() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("atmosphere.wjsl");

    assert!(
        wgsl.contains("@workgroup_size(8, 8, 1)"),
        "atmosphere must use 8x8 workgroup size to match pipeline dispatch\nWGSL:\n{}",
        wgsl
    );
}
