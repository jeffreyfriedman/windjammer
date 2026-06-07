#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Library `metadata.json` must carry converged `param_ownership`, not step-2 placeholders.
/// Cross-crate callers (game → engine) rely on Borrowed for read-only Vec params.
#[test]
fn test_library_metadata_json_includes_borrowed_param_ownership() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let renderer_dir = src.join("renderer");
    fs::create_dir_all(&renderer_dir).expect("mkdir");

    fs::write(
        renderer_dir.join("mod.wj"),
        r#"pub mod gpu;
"#,
    )
    .unwrap();

    fs::write(
        renderer_dir.join("gpu.wj"),
        r##"
pub struct Renderer {}

impl Renderer {
    pub fn upload_svo(self, svo_data: Vec<u32>, world_size: f32, depth: u32) {
        let _ = svo_data.len()
        let _ = world_size
        let _ = depth
    }
}
"##,
    )
    .unwrap();

    let out = tmp.path().join("gen");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");

    assert!(
        output.status.success(),
        "library build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata_path = out.join("metadata.json");
    let metadata = fs::read_to_string(&metadata_path).unwrap_or_else(|_| {
        panic!(
            "metadata.json missing at {}. stderr:\n{}",
            metadata_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
    });

    assert!(
        metadata.contains("Renderer::upload_svo"),
        "metadata should list Renderer::upload_svo. Got:\n{}",
        metadata
    );
    assert!(
        metadata.contains("Borrowed"),
        "metadata.json must include inferred Borrowed ownership for read-only Vec param. Got:\n{}",
        metadata
    );
}
