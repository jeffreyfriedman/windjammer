//! Shared WJSL shader fixtures for transpile tests.
//!
//! Shaders live under `tests/fixtures/shaders/` — no dependency on external repos.

#![allow(dead_code)]

use std::path::PathBuf;

pub fn shader_fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/shaders")
}

pub fn fixture_shader_path(filename: &str) -> PathBuf {
    shader_fixtures_dir().join(filename)
}

pub fn read_fixture_shader(filename: &str) -> Result<String, String> {
    std::fs::read_to_string(fixture_shader_path(filename))
        .map_err(|e| format!("Failed to read {filename}: {e}"))
}

pub fn transpile_fixture_shader(filename: &str) -> Result<String, String> {
    let base_dir = shader_fixtures_dir();
    let source = read_fixture_shader(filename)?;
    windjammer::wjsl::transpile_wjsl_with_includes(&source, &base_dir).map_err(|e| e.to_string())
}

/// Alias used by legacy shader quality tests.
pub fn transpile_shader_file(filename: &str) -> Result<String, String> {
    transpile_fixture_shader(filename)
}

pub fn transpile_fixture_shader_or_panic(filename: &str) -> String {
    transpile_fixture_shader(filename).unwrap_or_else(|e| panic!("{filename} should transpile: {e}"))
}
