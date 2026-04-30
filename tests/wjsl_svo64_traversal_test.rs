/// TDD: Verify voxel_raymarch.wjsl uses 64-tree (4x4x4) child indexing, not 8-tree (2x2x2).
///
/// Bug: SVO builder creates a 64-tree (64 children per node, 4x4x4 subdivision)
/// but the raymarch shader was using 8-tree octant logic (2x2x2), causing
/// complete SVO traversal failure and no voxels rendering.
///
/// Fix: Changed get_octant -> get_child_index_64 with proper quarter-based subdivision.
fn transpile(source: &str) -> Result<String, String> {
    windjammer::wjsl::transpile_wjsl(source).map_err(|e| e.to_string())
}

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
fn test_svo64_child_index_function_present() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let result = transpile_shader_file("voxel_raymarch.wjsl").unwrap();
    assert!(
        result.contains("get_child_index_64"),
        "Shader must use 64-tree child indexing function, not 8-tree get_octant"
    );
}

#[test]
fn test_svo64_no_octant_function() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let result = transpile_shader_file("voxel_raymarch.wjsl").unwrap();
    assert!(
        !result.contains("fn get_octant"),
        "Shader must NOT have 8-tree get_octant function. Use get_child_index_64 instead."
    );
}

#[test]
fn test_svo64_quarter_based_subdivision() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let result = transpile_shader_file("voxel_raymarch.wjsl").unwrap();
    assert!(
        result.contains("0.25"),
        "64-tree traversal must use quarter-based subdivision (node_size * 0.25)"
    );
}

#[test]
fn test_svo64_child_index_formula() {
    let source = r#"
fn get_child_index_64(p: vec3<f32>, node_min: vec3<f32>, node_size: f32) -> u32 {
    let quarter = node_size * 0.25;
    let rel = p - node_min;
    let xi = clamp(u32(rel.x / quarter), 0u, 3u);
    let yi = clamp(u32(rel.y / quarter), 0u, 3u);
    let zi = clamp(u32(rel.z / quarter), 0u, 3u);
    return xi + yi * 4u + zi * 16u;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = get_child_index_64(vec3(1.0, 2.0, 3.0), vec3(0.0), 4.0);
}
"#;
    let result = transpile(source).unwrap();
    assert!(
        result.contains("yi * 4u"),
        "Child index must use yi * 4u for 64-tree layout"
    );
    assert!(
        result.contains("zi * 16u"),
        "Child index must use zi * 16u for 64-tree layout"
    );
}

#[test]
fn test_lighting_shader_uses_svo64() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    assert!(
        result.contains("get_child_index_64"),
        "Lighting shader must also use 64-tree traversal to match SVO format"
    );
    assert!(
        !result.contains("fn get_octant"),
        "Lighting shader must NOT have 8-tree get_octant function"
    );
}

#[test]
fn test_lighting_shader_l1_cache_size_64() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let result = transpile_shader_file("voxel_lighting.wjsl").unwrap();
    assert!(
        result.contains("yi * 4u") && result.contains("zi * 16u"),
        "Lighting shader get_child_index_64 must use 4x4x4 (64-child) layout"
    );
}
