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

/// Owned formal with double-use body: converged Owned must win over body-inferred Borrowed.
#[test]
fn test_mannequin_generate_owned_formal_not_borrowed_at_call_site() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let character = src.join("character");
    let build = tmp.path().join("build");
    fs::create_dir_all(&character).expect("character dir");
    fs::create_dir_all(&build).expect("build");
    fs::write(src.join("mod.wj"), "mod character\n").expect("mod.wj");
    fs::write(character.join("mod.wj"), "mod mannequin\n").expect("character mod");
    fs::write(
        character.join("mannequin.wj"),
        r#"
pub struct MannequinConfig {
    pub height: f32,
    pub width: f32,
}

impl MannequinConfig {
    pub fn default() -> MannequinConfig {
        MannequinConfig { height: 1.8, width: 0.4 }
    }
}

pub struct MannequinMesh {
    pub vertex_count: i32,
}

impl MannequinMesh {
    pub fn generate(config: MannequinConfig) -> MannequinMesh {
        let h = config.height
        let w = config.width
        MannequinMesh { vertex_count: 100 }
    }
}

pub fn test_generation() {
    let config = MannequinConfig::default()
    let h = config.height
    let w = config.width
    let mesh = MannequinMesh::generate(config)
    assert_eq(mesh.vertex_count, 100)
}
"#,
    )
    .expect("mannequin.wj");

    let build_cmd = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            "--library",
            src.join("mod.wj").to_str().unwrap(),
            "--output",
            build.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");

    assert!(
        build_cmd.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&build_cmd.stderr)
    );

    let rs = fs::read_to_string(build.join("character/mannequin.rs")).expect("mannequin.rs");
    assert!(
        rs.contains("fn generate(config: MannequinConfig)"),
        "generate must take owned MannequinConfig. Got:\n{rs}"
    );
    assert!(
        !rs.contains("MannequinMesh::generate(&config)"),
        "owned formal must not receive &config at call site. Got:\n{rs}"
    );
    assert!(
        rs.contains("MannequinMesh::generate(config"),
        "call site must pass owned config (not &config). Got:\n{rs}"
    );
}
