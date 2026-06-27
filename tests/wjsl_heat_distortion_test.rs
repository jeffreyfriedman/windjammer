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


// TDD: heat_distortion.wjsl transpiles to valid WGSL (screen-space heat shimmer).
// Tests the heat distortion compute shader which produces realistic heat haze
// near fire sources, explosions, and hot surfaces.

#[test]
fn test_wjsl_heat_distortion_transpiles() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("heat_distortion.wjsl");

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
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("heat_distortion.wjsl");

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
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("heat_distortion.wjsl");

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
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("heat_distortion.wjsl");

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
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("heat_distortion.wjsl");

    assert!(
        wgsl.contains("@workgroup_size(8, 8, 1)"),
        "heat distortion must use 8x8 workgroup to match pipeline dispatch\nWGSL:\n{}",
        wgsl
    );
}
