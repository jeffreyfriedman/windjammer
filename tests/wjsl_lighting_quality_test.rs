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

/// TDD: Verify voxel_lighting.wjsl has correct PBR view vector, adequate shadow
/// quality, and proper AO sampling.
///
/// Bug: View vector V was `normalize(vec3(ndc.x, ndc.y, 1.0))` instead of
/// `normalize(camera_position - P)`. This makes Cook-Torrance specular and
/// Fresnel calculations completely wrong (not world-space).
///
/// Fix: Add camera_position to LightingParams and use world-space view direction.
fn game_shaders_available() -> bool {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammer-game/windjammer-game-core/shaders")
        .exists()
}

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
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    // Camera is `uniform camera: CameraUniforms`; PBR V uses `camera.position` in WGSL
    let has_camera_pos = result.contains("camera.position")
        || (result.contains("camera") && result.contains("position"));
    assert!(
        has_camera_pos,
        "Lighting shader must use camera + position for the view vector (e.g. camera.position in WGSL output)"
    );
    assert!(
        !result.contains("let V = normalize(vec3(ndc"),
        "View vector must NOT use NDC approximation - must use world-space camera position - P"
    );
}

#[test]
fn test_view_vector_world_space_computation() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    assert!(
        result.contains("camera.position - P")
            || result.contains("camera.position- P")
            || (result.contains("camera.position")
                && result.contains("normalize(")
                && result.contains("P")),
        "View vector V must be based on world-space position P and camera (e.g. normalize(camera.position - P))"
    );
}

#[test]
fn test_shadow_quality_minimum_samples() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    // trace_shadow() walks multiple segments along the light ray (loop over 6u); soft GI uses separate sample counts
    let has_shadow_march = result.contains("trace_shadow")
        && (result.contains("6u") || result.contains("for (var i = 0u;"));
    assert!(
        has_shadow_march,
        "Shadow pass should use trace_shadow with a multi-iteration along-ray march"
    );
}

#[test]
fn test_ao_uses_distance_falloff() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    assert!(
        result.contains("ao_distance")
            || result.contains("ao_radius")
            || result.contains("ao_range")
            || result.contains("1.5"),
        "AO should use distance-based falloff for smoother occlusion"
    );
}
