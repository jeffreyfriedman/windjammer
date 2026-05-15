//! TDD: water.wjsl transpiles to valid WGSL (animated water surface with reflections).
//! Tests the water compute shader which produces animated, reflective water surfaces
//! suitable for lakes, rivers, oceans in the voxel engine.

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
fn test_wjsl_water_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("water.wjsl");

    assert!(!wgsl.is_empty(), "transpiled WGSL must not be empty");
    assert!(
        wgsl.contains("@compute"),
        "output should declare @compute entry point"
    );
    assert!(
        wgsl.contains("WaterParams"),
        "output should define WaterParams struct"
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
fn test_water_binding_layout() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("water.wjsl");

    assert!(
        wgsl.contains("@binding(0)") && wgsl.contains("var<uniform> water_params"),
        "binding 0 must be WaterParams uniform\nWGSL:\n{}",
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
fn test_water_animation_features() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("water.wjsl");

    // Wave animation using time
    assert!(
        wgsl.contains("time"),
        "water must use time for animation\nWGSL:\n{}",
        wgsl
    );

    // Sine-based wave function
    assert!(
        wgsl.contains("sin("),
        "water must use sin() for wave animation\nWGSL:\n{}",
        wgsl
    );

    // Normal perturbation for reflections
    assert!(
        wgsl.contains("normalize("),
        "water must compute perturbed normals\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_water_reflection_features() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("water.wjsl");

    // Fresnel effect for reflection/refraction blending
    assert!(
        wgsl.contains("fresnel") || wgsl.contains("Fresnel") || wgsl.contains("pow("),
        "water must compute Fresnel term for reflection blending\nWGSL:\n{}",
        wgsl
    );

    // Reflection via reflect() or manual calculation
    assert!(
        wgsl.contains("reflect(") || wgsl.contains("reflect_dir"),
        "water must compute reflection direction\nWGSL:\n{}",
        wgsl
    );

    // Mix between water color and reflection
    assert!(
        wgsl.contains("mix("),
        "water must blend reflection with water color using mix()\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_water_workgroup_size() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("water.wjsl");

    assert!(
        wgsl.contains("@workgroup_size(8, 8, 1)"),
        "water must use 8x8 workgroup size to match pipeline dispatch\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_water_depth_and_transparency() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("water.wjsl");

    // Water depth affects transparency/absorption
    assert!(
        wgsl.contains("depth") || wgsl.contains("absorption"),
        "water must consider depth for transparency\nWGSL:\n{}",
        wgsl
    );

    // Water color absorption (deeper = darker/more blue)
    assert!(
        wgsl.contains("water_color") || wgsl.contains("deep_color"),
        "water must have configurable water color\nWGSL:\n{}",
        wgsl
    );
}
