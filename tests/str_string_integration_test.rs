#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Integration test: string params must not emit invalid bare `str` in Rust signatures.

use std::path::PathBuf;
use tempfile::TempDir;
use windjammer::build_project;
use windjammer::CompilationTarget;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_scene_manager_str_params_emit_ampersand_str() {
    let scene_manager_wj = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/str_string/scene_manager.wj");

    let temp_dir = TempDir::new().expect("temp dir");
    let output_dir = temp_dir.path();

    build_project(&scene_manager_wj, output_dir, CompilationTarget::Rust, true)
        .expect("build_project should succeed for fixture");

    let manager_rs = output_dir.join("scene_manager.rs");
    assert!(manager_rs.exists(), "scene_manager.rs should be generated");

    let content = std::fs::read_to_string(&manager_rs).expect("read scene_manager.rs");

    assert!(
        !content.contains("name: str)"),
        "Must not emit invalid 'name: str)' - str is unsized in Rust"
    );
}
