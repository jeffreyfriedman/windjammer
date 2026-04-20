/// TDD: Verify voxel_denoise.wjsl implements a proper 5x5 spatial filter
/// with neighborhood clamping and edge-aware weights.
///
/// Old: 3x3 filter (too small for GI noise), no neighborhood clamping
/// New: 5x5 edge-aware filter with temporal neighborhood clamping

fn transpile_shader_file(filename: &str) -> Result<String, String> {
    let base_dir = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../windjammer-game/windjammer-game-core/shaders"
    ));
    let path = base_dir.join(filename);
    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", filename, e))?;
    windjammer::wjsl::transpile_wjsl_with_includes(&source, &base_dir).map_err(|e| e.to_string())
}

#[test]
fn test_denoise_shader_transpiles() {
    let result = transpile_shader_file("voxel_denoise.wjsl");
    assert!(
        result.is_ok(),
        "Denoise shader must transpile: {:?}",
        result.err()
    );
}

#[test]
fn test_denoise_5x5_kernel() {
    let result = transpile_shader_file("voxel_denoise.wjsl").unwrap();
    assert!(
        result.contains("-2i") && result.contains("2i"),
        "Spatial filter must use 5x5 kernel (offsets -2 to +2)"
    );
}

#[test]
fn test_denoise_tile_size_144() {
    let result = transpile_shader_file("voxel_denoise.wjsl").unwrap();
    assert!(
        result.contains("144"),
        "Tile area must be 144 (12x12) for 5x5 kernel with 2-pixel border"
    );
}

#[test]
fn test_denoise_neighborhood_clamping() {
    let result = transpile_shader_file("voxel_denoise.wjsl").unwrap();
    assert!(
        result.contains("color_min")
            && result.contains("color_max")
            && result.contains("clamped_history"),
        "Temporal accumulation must use neighborhood clamping to prevent ghosting"
    );
}

#[test]
fn test_denoise_disocclusion_detection() {
    let result = transpile_shader_file("voxel_denoise.wjsl").unwrap();
    assert!(
        result.contains("depth_change") || result.contains("disocclusion"),
        "Denoise must detect disocclusion via depth changes"
    );
}
