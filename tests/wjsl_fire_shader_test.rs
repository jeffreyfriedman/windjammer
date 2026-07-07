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

#[path = "common/wjsl_shader_fixtures.rs"]
mod wjsl_shader_fixtures;

// TDD: Fire shader transpilation and validation tests

#[test]
fn test_fire_simulation_transpiles() {
    let result = wjsl_shader_fixtures::transpile_fixture_shader("fire_simulation.wjsl");
    assert!(
        result.is_ok(),
        "fire_simulation.wjsl should transpile. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_fire_render_transpiles() {
    let result = wjsl_shader_fixtures::transpile_fixture_shader("fire_render.wjsl");
    assert!(
        result.is_ok(),
        "fire_render.wjsl should transpile. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_fire_simulation_workgroup_size() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("fire_simulation.wjsl").unwrap();
    assert!(
        wgsl.contains("@workgroup_size(4, 4, 4)"),
        "Fire simulation should use 4x4x4 workgroup for 3D grid. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_fire_simulation_bindings() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("fire_simulation.wjsl").unwrap();
    assert!(
        wgsl.contains("fire_params"),
        "Should have fire_params uniform binding"
    );
    assert!(
        wgsl.contains("temperature_grid"),
        "Should have temperature_grid storage binding"
    );
}

#[test]
fn test_fire_simulation_noise() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("fire_simulation.wjsl").unwrap();
    assert!(
        wgsl.contains("noise3d") || wgsl.contains("fbm"),
        "Fire simulation should use noise functions for turbulence. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_fire_render_blackbody() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("fire_render.wjsl").unwrap();
    assert!(
        wgsl.contains("blackbody_color"),
        "Fire render should include blackbody_color function. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_fire_render_bindings() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("fire_render.wjsl").unwrap();
    assert!(
        wgsl.contains("fire_render_params"),
        "Should have fire_render_params uniform"
    );
    assert!(
        wgsl.contains("temperature_grid"),
        "Should have temperature_grid storage (read)"
    );
    assert!(
        wgsl.contains("color_output"),
        "Should have color_output storage (read_write)"
    );
}

#[test]
fn test_fire_render_camera() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("fire_render.wjsl").unwrap();
    assert!(
        wgsl.contains("CameraUniforms"),
        "Fire render should use CameraUniforms from wjsl_std. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_fire_render_raymarching() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("fire_render.wjsl").unwrap();
    assert!(
        wgsl.contains("intersect_box"),
        "Should have ray-box intersection. Got:\n{}",
        wgsl
    );
    assert!(
        wgsl.contains("transmittance"),
        "Should track transmittance for Beer-Lambert. Got:\n{}",
        wgsl
    );
}
