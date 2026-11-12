//! Declared path dependencies (`CompilerConfig::crate_paths`) drive generated Cargo.toml.

use std::collections::HashSet;
use windjammer::cargo_integration::create_cargo_toml_with_deps;
use windjammer::compiler_config::CompilerConfig;
use windjammer::CompilationTarget;

fn manifest(config: &CompilerConfig, crates: &[&str]) -> String {
    let out = tempfile::TempDir::new().unwrap();
    let src = tempfile::TempDir::new().unwrap();
    let crates: Vec<String> = crates.iter().map(|s| s.to_string()).collect();
    create_cargo_toml_with_deps(
        out.path(),
        &HashSet::new(),
        &crates,
        CompilationTarget::Rust,
        src.path(),
        config,
    )
    .unwrap();
    std::fs::read_to_string(out.path().join("Cargo.toml")).unwrap()
}

#[test]
fn declared_crate_path_becomes_a_path_dependency() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut config = CompilerConfig::default();
    config.crate_paths.insert("acme-core".into(), dir.path().to_path_buf());
    let toml = manifest(&config, &["acme_core"]);
    assert!(toml.contains("acme-core = { path = "), "{toml}");
}

#[test]
fn undeclared_or_missing_crate_path_falls_back_to_registry() {
    let mut config = CompilerConfig::default();
    config.crate_paths.insert("acme-core".into(), "/definitely/not/here".into());
    let toml = manifest(&config, &["acme_core", "other_crate"]);
    assert!(toml.contains("other_crate = \"*\""), "{toml}");
    assert!(!toml.contains("path = \"/definitely"), "{toml}");
}
