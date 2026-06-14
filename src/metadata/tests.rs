use super::*;
use crate::analyzer::OwnershipMode;

fn create_project(base: &std::path::Path, subdirs: &[&str]) {
    std::fs::create_dir_all(base).unwrap();
    std::fs::write(base.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
    for sub in subdirs {
        std::fs::create_dir_all(base.join(sub)).unwrap();
    }
}

#[test]
fn test_metadata_round_trip() {
    let mut meta = ModuleMetadata::new("math::vec3".to_string());

    meta.functions.insert(
        "Vec3::new".to_string(),
        FunctionSignature {
            params: vec![
                "Custom(\"f32\")".to_string(),
                "Custom(\"f32\")".to_string(),
                "Custom(\"f32\")".to_string(),
            ],
            return_type: Some("Custom(\"Vec3\")".to_string()),
            is_associated: true,
            parent_type: Some("Vec3".to_string()),
            param_ownership: vec![],
            has_self_receiver: false,
            is_extern: false,
        },
    );

    let json = serde_json::to_string_pretty(&meta).unwrap();
    let loaded: ModuleMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.functions.len(), 1);
    assert!(loaded.functions.contains_key("Vec3::new"));
}

#[test]
fn test_meta_cache_path_src() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("myproject");
    create_project(&proj, &["src/math"]);
    let source = proj.join("src/math/vec3.wj");
    std::fs::write(&source, "").unwrap();

    let result = meta_cache_path(&source);
    assert_eq!(result, proj.join(".wj-cache/math/vec3.wj.meta"));
}

#[test]
fn test_meta_cache_path_nested() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("myproject");
    create_project(&proj, &["src/rendering/shaders"]);
    let source = proj.join("src/rendering/shaders/mesh.wj");
    std::fs::write(&source, "").unwrap();

    let result = meta_cache_path(&source);
    assert_eq!(
        result,
        proj.join(".wj-cache/rendering/shaders/mesh.wj.meta")
    );
}

#[test]
fn test_meta_cache_path_top_level() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("myproject");
    create_project(&proj, &["src"]);
    let source = proj.join("src/main.wj");
    std::fs::write(&source, "").unwrap();

    let result = meta_cache_path(&source);
    assert_eq!(result, proj.join(".wj-cache/main.wj.meta"));
}

#[test]
fn test_meta_cache_path_no_project_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("noproject");
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("file.wj");
    std::fs::write(&source, "").unwrap();

    let result = meta_cache_path(&source);
    assert_eq!(result, dir.join("file.wj.meta"));
}

#[test]
fn test_meta_cache_path_components_subdir() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("uiproject");
    create_project(&proj, &["src/components"]);
    let source = proj.join("src/components/textarea.wj");
    std::fs::write(&source, "").unwrap();

    let result = meta_cache_path(&source);
    assert_eq!(result, proj.join(".wj-cache/components/textarea.wj.meta"));
}

#[test]
fn test_meta_cache_path_components_nested() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("uiproject");
    create_project(&proj, &["src/components/forms"]);
    let source = proj.join("src/components/forms/input.wj");
    std::fs::write(&source, "").unwrap();

    let result = meta_cache_path(&source);
    assert_eq!(
        result,
        proj.join(".wj-cache/components/forms/input.wj.meta")
    );
}

#[test]
fn test_meta_cache_root_with_cargo_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("myproject");
    create_project(&proj, &["src"]);
    let src = proj.join("src");

    let result = meta_cache_root(&src);
    assert_eq!(result, proj.join(".wj-cache"));
}

#[test]
fn test_meta_cache_root_no_project() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("noproject/src");
    std::fs::create_dir_all(&dir).unwrap();

    let result = meta_cache_root(&dir);
    assert_eq!(result, dir.join(".wj-cache"));
}

#[test]
fn test_meta_cache_root_wj_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("wjproject");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(proj.join("wj.toml"), "[project]\nname = \"test\"").unwrap();
    std::fs::create_dir_all(proj.join("src")).unwrap();
    let src = proj.join("src");

    let result = meta_cache_root(&src);
    assert_eq!(result, proj.join(".wj-cache"));
}

#[test]
fn test_project_root_metadata_overrides_stale_wj_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("gameproject");
    create_project(&proj, &["src/rendering", ".wj-cache/rendering"]);

    // Stale .wj-cache entry with WRONG ownership (MutBorrowed for param that should be Owned)
    let stale_meta = ModuleMetadata {
        module_path: "voxel_renderer".to_string(),
        functions: {
            let mut fns = HashMap::new();
            fns.insert(
                "VoxelRenderer::set_camera".to_string(),
                FunctionSignature {
                    params: vec![
                        "Custom(\"VoxelRenderer\")".to_string(),
                        "Custom(\"CameraData\")".to_string(),
                    ],
                    return_type: None,
                    is_associated: true,
                    parent_type: Some("VoxelRenderer".to_string()),
                    param_ownership: vec!["Borrowed".to_string(), "MutBorrowed".to_string()],
                    has_self_receiver: true,
                    is_extern: false,
                },
            );
            fns
        },
        structs: HashMap::new(),
        trait_impls: HashMap::new(),
        copy_structs: vec![],
        non_copy_structs: vec![],
        version: "0.1.0".to_string(),
        analysis_fingerprint: None,
    };
    let cache_path = proj.join(".wj-cache/rendering/voxel_renderer.wj.meta");
    std::fs::write(&cache_path, serde_json::to_string(&stale_meta).unwrap()).unwrap();

    // Correct project-root metadata.json with RIGHT ownership
    let correct_meta = CrateMetadata {
        structs: HashMap::new(),
        functions: {
            let mut fns = HashMap::new();
            fns.insert(
                "VoxelRenderer::set_camera".to_string(),
                FunctionSignature {
                    params: vec![
                        "Custom(\"VoxelRenderer\")".to_string(),
                        "Custom(\"CameraData\")".to_string(),
                    ],
                    return_type: None,
                    is_associated: true,
                    parent_type: Some("VoxelRenderer".to_string()),
                    param_ownership: vec!["MutBorrowed".to_string(), "Owned".to_string()],
                    has_self_receiver: true,
                    is_extern: false,
                },
            );
            fns
        },
        version: "0.1.0".to_string(),
    };
    std::fs::write(
        proj.join("metadata.json"),
        serde_json::to_string(&correct_meta).unwrap(),
    )
    .unwrap();

    let mut registry = crate::analyzer::SignatureRegistry::new();
    let mut analyzer = crate::analyzer::Analyzer::new();
    merge_wj_meta_signatures_and_copy_structs(&proj.join("src"), &mut registry, &mut analyzer);

    let sig = registry
        .get_signature("VoxelRenderer::set_camera")
        .expect("signature should exist");
    assert_eq!(
        sig.param_ownership,
        vec![OwnershipMode::MutBorrowed, OwnershipMode::Owned],
        "project-root metadata.json should override stale .wj-cache entry"
    );
    assert!(sig.has_self_receiver);
}
