/// TDD: Fire shader transpilation and validation tests
use std::fs;
use std::path::Path;

fn read_shader_source(filename: &str) -> (String, std::path::PathBuf) {
    let shader_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammer-game/windjammer-game-core/shaders");
    let shader_path = shader_dir.join(filename);
    let source = fs::read_to_string(&shader_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));
    (source, shader_dir)
}

fn transpile_shader(filename: &str) -> Result<String, String> {
    let (source, shader_dir) = read_shader_source(filename);
    windjammer::wjsl::transpile_wjsl_with_includes(&source, &shader_dir).map_err(|e| e.to_string())
}

#[test]
fn test_fire_simulation_transpiles() {
    let result = transpile_shader("fire_simulation.wjsl");
    assert!(
        result.is_ok(),
        "fire_simulation.wjsl should transpile. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_fire_render_transpiles() {
    let result = transpile_shader("fire_render.wjsl");
    assert!(
        result.is_ok(),
        "fire_render.wjsl should transpile. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_fire_simulation_workgroup_size() {
    let wgsl = transpile_shader("fire_simulation.wjsl").unwrap();
    assert!(
        wgsl.contains("@workgroup_size(4, 4, 4)"),
        "Fire simulation should use 4x4x4 workgroup for 3D grid. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_fire_simulation_bindings() {
    let wgsl = transpile_shader("fire_simulation.wjsl").unwrap();
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
    let wgsl = transpile_shader("fire_simulation.wjsl").unwrap();
    assert!(
        wgsl.contains("noise3d") || wgsl.contains("fbm"),
        "Fire simulation should use noise functions for turbulence. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_fire_render_blackbody() {
    let wgsl = transpile_shader("fire_render.wjsl").unwrap();
    assert!(
        wgsl.contains("blackbody_color"),
        "Fire render should include blackbody_color function. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_fire_render_bindings() {
    let wgsl = transpile_shader("fire_render.wjsl").unwrap();
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
    let wgsl = transpile_shader("fire_render.wjsl").unwrap();
    assert!(
        wgsl.contains("CameraUniforms"),
        "Fire render should use CameraUniforms from wjsl_std. Got:\n{}",
        wgsl
    );
}

#[test]
fn test_fire_render_raymarching() {
    let wgsl = transpile_shader("fire_render.wjsl").unwrap();
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
