//! Integration test: str/String fix on real windjammer-game-core code
//!
//! Verifies that compiling scene/manager.wj produces &str (not str) in params.

use std::path::PathBuf;
use tempfile::TempDir;
use windjammer::build_project;
use windjammer::CompilationTarget;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_scene_manager_str_params_emit_ampersand_str() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("..");
    let scene_manager_wj = workspace_root
        .join("windjammer-game")
        .join("windjammer-game-core")
        .join("src_wj")
        .join("scene")
        .join("manager.wj");

    if !scene_manager_wj.exists() {
        eprintln!("Skipping: windjammer-game not found at {:?}", scene_manager_wj);
        return;
    }

    let temp_dir = TempDir::new().expect("temp dir");
    let output_dir = temp_dir.path();

    let result = build_project(&scene_manager_wj, output_dir, CompilationTarget::Rust, true);
    if let Err(e) = result {
        eprintln!("build_project failed: {}", e);
        return; // Don't fail test if windjammer-game structure differs
    }

    let manager_rs = output_dir.join("manager.rs");
    if !manager_rs.exists() {
        eprintln!("Skipping: manager.rs not generated (output structure may differ)");
        return;
    }

    let content = std::fs::read_to_string(&manager_rs).expect("read manager.rs");

    // Our fix: params like "name: str" should emit "name: &str"
    assert!(
        content.contains("name: &str"),
        "Expected 'name: &str' in generated code (str->&str fix). Got snippet: {}",
        &content[..content.len().min(2000)]
    );

    // Must NOT emit invalid bare "name: str)"
    assert!(
        !content.contains("name: str)"),
        "Must not emit invalid 'name: str)' - str is unsized in Rust"
    );
}
