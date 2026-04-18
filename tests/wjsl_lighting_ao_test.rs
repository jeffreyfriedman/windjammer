// TDD Test: Voxel Lighting with Ambient Occlusion and PBR
//
// Verifies that the enhanced voxel_lighting.wjsl shader:
// 1. Transpiles successfully from WJSL to WGSL
// 2. Contains AO sampling functions
// 3. Contains PBR (Cook-Torrance) lighting functions
// 4. Contains shadow ray tracing
// 5. Uses roughness/metallic from material palette

use std::fs;
use std::path::Path;

fn transpile(wjsl_path: &str) -> Result<String, String> {
    let source = fs::read_to_string(wjsl_path)
        .map_err(|e| format!("Failed to read {}: {}", wjsl_path, e))?;
    let base_dir = Path::new(wjsl_path).parent().unwrap_or(Path::new("."));
    windjammer::wjsl::transpile_wjsl_with_includes(&source, base_dir)
        .map_err(|e| format!("WJSL transpilation failed: {}", e))
}

#[test]
fn test_lighting_shader_transpiles() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/voxel_lighting.wjsl")
        .expect("voxel_lighting.wjsl should transpile");
    assert!(!wgsl.is_empty(), "Generated WGSL should not be empty");
    assert!(wgsl.contains("fn main"), "Should contain main entry point");
}

#[test]
fn test_lighting_has_ao_sampling() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/voxel_lighting.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("compute_ao") || wgsl.contains("ambient_occlusion") || wgsl.contains("ao_factor"),
        "Lighting shader should contain AO computation. Generated:\n{}",
        &wgsl[..wgsl.len().min(2000)]
    );
}

#[test]
fn test_lighting_has_pbr_specular() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/voxel_lighting.wjsl")
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
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/voxel_lighting.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("trace_shadow") || wgsl.contains("shadow_factor"),
        "Lighting shader should trace shadow rays"
    );
}

#[test]
fn test_lighting_has_sky_gradient() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/voxel_lighting.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("sky_color") || wgsl.contains("ground_color"),
        "Lighting shader should have sky gradient for ambient"
    );
}
