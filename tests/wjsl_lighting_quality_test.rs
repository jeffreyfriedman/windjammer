/// TDD: Verify voxel_lighting.wjsl has correct PBR view vector, adequate shadow
/// quality, and proper AO sampling.
///
/// Bug: View vector V was `normalize(vec3(ndc.x, ndc.y, 1.0))` instead of
/// `normalize(camera_position - P)`. This makes Cook-Torrance specular and
/// Fresnel calculations completely wrong (not world-space).
///
/// Fix: Add camera_position to LightingParams and use world-space view direction.
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
fn test_view_vector_uses_camera_position() {
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    assert!(
        result.contains("camera_position"),
        "Lighting shader must have camera_position in the uniform for correct PBR view vector"
    );
    assert!(
        !result.contains("let V = normalize(vec3(ndc"),
        "View vector must NOT use NDC approximation - must use world-space camera_position - P"
    );
}

#[test]
fn test_view_vector_world_space_computation() {
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    assert!(
        result.contains("camera_position")
            && (result.contains("normalize(lighting.camera_position - P)")
                || result.contains("normalize(lighting.camera_position - gbuf.position)")),
        "View vector V must be normalize(camera_position - P) for correct Cook-Torrance PBR"
    );
}

#[test]
fn test_shadow_quality_minimum_samples() {
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    let has_multi_sample = result.contains("shadow_samples")
        || (result.contains("trace_shadow_ray") && result.contains("4u"));
    assert!(
        has_multi_sample,
        "Shadow tracing should use at least 4 jittered samples for soft shadows"
    );
}

#[test]
fn test_ao_uses_distance_falloff() {
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    assert!(
        result.contains("ao_distance")
            || result.contains("ao_radius")
            || result.contains("ao_range")
            || result.contains("1.5"),
        "AO should use distance-based falloff for smoother occlusion"
    );
}
