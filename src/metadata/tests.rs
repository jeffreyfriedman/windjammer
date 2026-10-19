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
fn test_skeleton_param_ownership_defaults() {
    use crate::parser::Type;

    let owned = default_skeleton_param_ownership_from_types(&[Type::Custom("QuestId".into())]);
    assert_eq!(owned, vec!["Owned"]);

    let borrowed = default_skeleton_param_ownership_from_types(&[Type::String]);
    assert_eq!(borrowed, vec!["Borrowed"]);

    let mixed = default_skeleton_param_ownership_from_types(&[
        Type::String,
        Type::Custom("MannequinConfig".into()),
    ]);
    assert_eq!(mixed, vec!["Borrowed", "Owned"]);
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
            formal_params: vec![],
            return_type: Some("Custom(\"Vec3\")".to_string()),
            is_associated: true,
            parent_type: Some("Vec3".to_string()),
            param_ownership: vec![],
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            forwarding_borrow_params: None,
            has_self_receiver: false,
            is_extern: false,
            is_trait_method: false,
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
                    formal_params: vec![],
                    return_type: None,
                    is_associated: true,
                    parent_type: Some("VoxelRenderer".to_string()),
                    param_ownership: vec!["Borrowed".to_string(), "MutBorrowed".to_string()],
                    emitted_rust_ref_params: None,
                    string_ref_string_formal_params: None,
                    forwarding_borrow_params: None,
                    has_self_receiver: true,
                    is_extern: false,
                    is_trait_method: false,
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
                    formal_params: vec![],
                    return_type: None,
                    is_associated: true,
                    parent_type: Some("VoxelRenderer".to_string()),
                    param_ownership: vec!["MutBorrowed".to_string(), "Owned".to_string()],
                    emitted_rust_ref_params: None,
                    string_ref_string_formal_params: None,
                    forwarding_borrow_params: None,
                    has_self_receiver: true,
                    is_extern: false,
                    is_trait_method: false,
                },
            );
            fns
        },
        version: "0.1.0".to_string(),
        copy_structs: Vec::new(),
        string_consts: Vec::new(),
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

#[test]
fn stdlib_meta_string_file_deserializes_without_trait_impls_or_version() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("stdlib_meta/string.wj.meta");
    let text = std::fs::read_to_string(&path).expect("read string.wj.meta");
    let meta: crate::metadata::ModuleMetadata =
        serde_json::from_str(&text).expect("stdlib_meta/string.wj.meta must deserialize");
    assert!(
        meta.functions.contains_key("String::find"),
        "String::find must be present for Pattern/&str call-site borrowing"
    );
    assert!(meta.functions.contains_key("String::chars"));
}

fn meta_with_fingerprint(compiler: String, content: u64) -> ModuleMetadata {
    let mut m = ModuleMetadata::new("m".to_string());
    m.analysis_fingerprint = Some(AnalysisFingerprint {
        content_hash: content,
        compiler_version: compiler,
        dep_metadata_epoch: 0,
        output_hash: 0,
    });
    m
}

#[test]
fn wj_meta_sidecar_is_trusted_only_when_current() {
    use crate::compiler::incremental::{compiler_build_identity, hash_source};
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("m.wj");
    std::fs::write(&src, "fn f() {}\n").unwrap();
    let meta_path = dir.path().join("m.wj.meta");
    let now = compiler_build_identity();
    let h = hash_source("fn f() {}\n");

    assert!(wj_meta_is_current(&meta_path, &meta_with_fingerprint(now.clone(), h)), "matching sidecar is current");
    assert!(
        !wj_meta_is_current(&meta_path, &meta_with_fingerprint(now.clone(), h ^ 1)),
        "source text changed since the sidecar was written: stale"
    );
    assert!(
        !wj_meta_is_current(&meta_path, &meta_with_fingerprint("other-build".into(), h)),
        "written by another compiler build: stale"
    );
    assert!(
        wj_meta_is_current(&meta_path, &ModuleMetadata::new("m".into())),
        "sidecars without a fingerprint (stdlib / hand-authored) are accepted"
    );
    // No sibling source (metadata shipped without sources): the compiler identity still gates it.
    std::fs::remove_file(&src).unwrap();
    assert!(wj_meta_is_current(&meta_path, &meta_with_fingerprint(now, h ^ 1)));
}

#[test]
fn stale_sidecar_registers_no_signatures() {
    use crate::compiler::incremental::compiler_build_identity;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("m.wj"), "fn f() {}\n").unwrap();
    let mut meta = meta_with_fingerprint(compiler_build_identity(), 12345); // wrong content hash
    meta.functions.insert(
        "stale_fn".to_string(),
        serde_json::from_str(r#"{"params":[],"return_type":null,"is_associated":false,"parent_type":null}"#).unwrap(),
    );
    std::fs::write(dir.path().join("m.wj.meta"), serde_json::to_string(&meta).unwrap()).unwrap();
    let mut registry = crate::analyzer::SignatureRegistry::new();
    merge_wj_meta_signatures_from_dir_only(dir.path(), &mut registry);
    assert!(registry.get_signature("stale_fn").is_none(), "a stale sidecar must not feed the registry");
}
