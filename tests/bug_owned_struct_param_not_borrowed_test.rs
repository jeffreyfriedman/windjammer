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

/// Static method that consumes a non-Copy struct must receive owned value at call site,
/// not `&config`, even when stale engine metadata lists bare `Custom(T)`.
#[test]
fn test_owned_struct_param_passed_by_value_not_ref() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("mannequin.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir");

    fs::write(
        &wj,
        r##"
pub struct MannequinConfig {
    pub torso_height: f32,
}

impl MannequinConfig {
    pub fn default_config() -> MannequinConfig {
        MannequinConfig { torso_height: 1.0 }
    }
}

pub struct MannequinMesh {
    tag: i32,
}

impl MannequinMesh {
    pub fn generate(config: MannequinConfig) -> MannequinMesh {
        let _ = config.torso_height
        MannequinMesh { tag: 1 }
    }
}

pub fn test_mannequin_default_generation() {
    let config = MannequinConfig::default_config()
    let mesh = MannequinMesh::generate(config)
    assert_eq(mesh.tag, 1)
}
"##,
    )
    .unwrap();

    let stub_meta = tmp.path().join("stub_meta.json");
    fs::write(
        &stub_meta,
        r##"{
  "functions": {
    "MannequinMesh::generate": {
      "params": ["Custom(\"MannequinConfig\")"],
      "return_type": "Custom(\"MannequinMesh\")",
      "is_associated": true,
      "parent_type": "MannequinMesh",
      "param_ownership": [],
      "has_self_receiver": false,
      "is_extern": false
    }
  }
}"##,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("engine={}", stub_meta.display()),
        ])
        .output()
        .expect("wj build");

    assert!(
        build.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("mannequin.rs")).expect("mannequin.rs");
    assert!(
        rs.contains("fn generate(config: MannequinConfig)"),
        "generate must take owned MannequinConfig. Got:\n{rs}"
    );
    assert!(
        !rs.contains("MannequinMesh::generate(&config)"),
        "owned consumer must not receive &config. Got:\n{rs}"
    );
    assert!(
        rs.contains("MannequinMesh::generate(config")
            || rs.contains("MannequinMesh::generate(config.clone()"),
        "call site must pass owned config. Got:\n{rs}"
    );
}
