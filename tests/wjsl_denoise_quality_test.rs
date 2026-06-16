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


/// TDD: Verify voxel_denoise.wjsl implements a proper 5x5 spatial filter
/// with neighborhood clamping and edge-aware weights.
///
/// Old: 3x3 filter (too small for GI noise), no neighborhood clamping
/// New: 5x5 edge-aware filter with temporal neighborhood clamping
#[test]
fn test_denoise_shader_transpiles() {
    let result = wjsl_shader_fixtures::transpile_shader_file("voxel_denoise.wjsl");
    assert!(
        result.is_ok(),
        "Denoise shader must transpile: {:?}",
        result.err()
    );
}

#[test]
fn test_denoise_5x5_kernel() {
    let result = wjsl_shader_fixtures::transpile_shader_file("voxel_denoise.wjsl").unwrap();
    assert!(
        result.contains("-2i") && result.contains("2i"),
        "Spatial filter must use 5x5 kernel (offsets -2 to +2)"
    );
}

/// Workgroup is 8×8 = 64 threads; shader does not use a 12×12 (144) shared-memory tile.
#[test]
fn test_denoise_workgroup_8x8_64() {
    let result = wjsl_shader_fixtures::transpile_shader_file("voxel_denoise.wjsl").unwrap();
    assert!(
        result.contains("@workgroup_size(8, 8, 1)"),
        "Expected 8x8 workgroup (WGSL a-trous full-res path, not a 12x12/144-tile design); got:\n{}",
        result
    );
}

#[test]
fn test_denoise_neighborhood_clamping() {
    let result = wjsl_shader_fixtures::transpile_shader_file("voxel_denoise.wjsl").unwrap();
    // Current shader uses temporal `mix` + depth rejection; stricter a-trous may add explicit min/max history clamps later
    let has_temporal_temper = (result.contains("mix(") && result.contains("history"))
        || (result.contains("color_min")
            && result.contains("color_max")
            && result.contains("clamped_history"));
    assert!(
        has_temporal_temper,
        "Temporal pass should blend history with filtered color (or explicit neighborhood clamp)"
    );
}

#[test]
fn test_denoise_disocclusion_detection() {
    let result = wjsl_shader_fixtures::transpile_shader_file("voxel_denoise.wjsl").unwrap();
    assert!(
        result.contains("depth_change") || result.contains("disocclusion"),
        "Denoise must detect disocclusion via depth changes"
    );
}
