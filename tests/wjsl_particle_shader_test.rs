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


// TDD: particle_simulation.wjsl and particle_render.wjsl transpile to valid WGSL.
// Tests the GPU particle system compute shaders for simulation and rendering.

#[test]
fn test_particle_simulation_transpiles() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("particle_simulation.wjsl");

    assert!(!wgsl.is_empty());
    assert!(wgsl.contains("@compute"), "should have compute entry point");
    assert!(wgsl.contains("Particle"), "should define Particle struct");
    assert!(
        wgsl.contains("ParticleParams"),
        "should define ParticleParams"
    );
    assert!(
        wgsl.contains("ParticleEmitterUniforms"),
        "should define emitter uniforms"
    );
    assert!(
        wgsl.contains("workgroup_size(64"),
        "should use 64-wide workgroup"
    );
}

#[test]
fn test_particle_simulation_bindings() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("particle_simulation.wjsl");

    assert!(
        wgsl.contains("@binding(0)") && wgsl.contains("particles"),
        "binding 0 must be particles storage\nWGSL:\n{}",
        wgsl
    );
    assert!(
        wgsl.contains("@binding(1)") && wgsl.contains("params"),
        "binding 1 must be ParticleParams uniform\nWGSL:\n{}",
        wgsl
    );
    assert!(
        wgsl.contains("@binding(2)") && wgsl.contains("emitter"),
        "binding 2 must be emitter uniform\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_particle_simulation_physics() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("particle_simulation.wjsl");

    assert!(
        wgsl.contains("gravity"),
        "simulation must apply gravity\nWGSL:\n{}",
        wgsl
    );
    assert!(
        wgsl.contains("wind"),
        "simulation must apply wind\nWGSL:\n{}",
        wgsl
    );
    assert!(
        wgsl.contains("delta_time"),
        "simulation must use delta_time\nWGSL:\n{}",
        wgsl
    );
    assert!(
        wgsl.contains("turbulence"),
        "simulation must support turbulence\nWGSL:\n{}",
        wgsl
    );
}

#[test]
fn test_particle_render_transpiles() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("particle_render.wjsl");

    assert!(!wgsl.is_empty());
    assert!(wgsl.contains("@compute"), "should have compute entry point");
    assert!(wgsl.contains("Particle"), "should define Particle struct");
    assert!(
        wgsl.contains("CameraUniforms"),
        "should define CameraUniforms"
    );
    assert!(
        wgsl.contains("ParticleRenderParams"),
        "should define render params"
    );
    assert!(
        wgsl.contains("workgroup_size(64"),
        "should use 64-wide workgroup"
    );
}

#[test]
fn test_particle_render_bindings() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("particle_render.wjsl");

    assert!(
        wgsl.contains("@binding(0)") && wgsl.contains("particles"),
        "binding 0 must be particles storage read"
    );
    assert!(
        wgsl.contains("@binding(1)") && wgsl.contains("camera"),
        "binding 1 must be camera uniform"
    );
    assert!(
        wgsl.contains("@binding(2)") && wgsl.contains("render_params"),
        "binding 2 must be render params"
    );
    assert!(
        wgsl.contains("@binding(3)") && wgsl.contains("gbuffer"),
        "binding 3 must be gbuffer"
    );
    assert!(
        wgsl.contains("@binding(4)") && wgsl.contains("color_output"),
        "binding 4 must be color_output"
    );
}

#[test]
fn test_particle_render_projection() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader_or_panic("particle_render.wjsl");

    assert!(
        wgsl.contains("project_world_to_clip"),
        "must project particles to screen space"
    );
    assert!(
        wgsl.contains("scene_depth_linear"),
        "must read scene depth for occlusion"
    );
    assert!(
        wgsl.contains("splat_contrib"),
        "must compute soft splat contribution"
    );
}
