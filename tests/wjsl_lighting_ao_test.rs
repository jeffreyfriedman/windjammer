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


// TDD Test: Voxel Lighting with Ambient Occlusion and PBR
//
// Verifies that the enhanced voxel_lighting.wjsl shader:
// 1. Transpiles successfully from WJSL to WGSL
// 2. Contains AO sampling functions
// 3. Contains PBR (Cook-Torrance) lighting functions
// 4. Contains shadow ray tracing
// 5. Uses roughness/metallic from material palette


#[test]
fn test_lighting_shader_transpiles() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("voxel_lighting.wjsl")
        .expect("voxel_lighting.wjsl should transpile");
    assert!(!wgsl.is_empty(), "Generated WGSL should not be empty");
    assert!(wgsl.contains("fn main"), "Should contain main entry point");
}

#[test]
fn test_lighting_has_ao_sampling() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("voxel_lighting.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("compute_ao")
            || wgsl.contains("ambient_occlusion")
            || wgsl.contains("ao_factor"),
        "Lighting shader should contain AO computation. Generated:\n{}",
        &wgsl[..wgsl.len().min(2000)]
    );
}

#[test]
fn test_lighting_has_pbr_specular() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("voxel_lighting.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("roughness") && wgsl.contains("metallic"),
        "Lighting shader should use roughness and metallic for PBR"
    );
    assert!(
        wgsl.contains("specular") || wgsl.contains("ggx") || wgsl.contains("cook_torrance"),
        "Lighting shader should compute specular reflection"
    );
}

#[test]
fn test_lighting_has_shadow_rays() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("voxel_lighting.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("trace_shadow") || wgsl.contains("shadow_factor"),
        "Lighting shader should trace shadow rays"
    );
}

#[test]
fn test_lighting_has_sky_gradient() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("voxel_lighting.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("sky_color") || wgsl.contains("ground_color"),
        "Lighting shader should have sky gradient for ambient"
    );
}
