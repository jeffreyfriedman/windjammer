#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

//! TDD: Shader files should be detected by content, not directory name.
//!
//! Bug: The compiler hardcodes `if dir_name != "shaders"` to skip shader files.
//! This leaks game-specific knowledge into the compiler.
//!
//! Fix: Detect shader files by their content (@vertex, @fragment, @compute decorators)
//! and skip them from the Rust compilation pipeline regardless of directory name.

use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_shader_files_excluded_from_rust_pipeline_by_content() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let build = dir.path().join("build");
    fs::create_dir_all(&src).unwrap();

    // mod.wj declares both a regular module and a shader module
    fs::write(src.join("mod.wj"), "pub mod game\npub mod my_shader\n").unwrap();

    // Regular game code
    fs::write(
        src.join("game.wj"),
        r#"
pub struct Player {
    x: float,
    y: float,
}
"#,
    )
    .unwrap();

    // Shader file (uses @vertex decorator) - NOT in a "shaders" directory
    fs::write(
        src.join("my_shader.wj"),
        r#"
pub struct Uniforms {
    mvp: mat4x4<float>,
}

@group(0) @binding(0) @uniform
extern let uniforms: Uniforms

@vertex
pub fn vs_main(@location(0) pos: vec3<float>) -> vec4<float> {
    uniforms.mvp * vec4(pos.x, pos.y, pos.z, 1.0)
}
"#,
    )
    .unwrap();

    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--output",
            build.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "Build should succeed (shader files skipped from Rust pipeline).\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // game.rs should exist (regular code compiled)
    assert!(
        build.join("game.rs").exists(),
        "game.rs should be generated from game.wj"
    );

    // my_shader.rs should NOT exist (shader file detected and skipped)
    // OR if it exists, it should not contain invalid Rust types like vec3<f64>
    if build.join("my_shader.rs").exists() {
        let content = fs::read_to_string(build.join("my_shader.rs")).unwrap();
        assert!(
            !content.contains("vec3<f64>") && !content.contains("mat4x4<f64>"),
            "Shader file should not generate invalid Rust types.\nGenerated:\n{}",
            content
        );
    }
}

#[test]
fn test_shader_in_any_directory_detected_by_content() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let effects_dir = src.join("effects");
    let build = dir.path().join("build");
    fs::create_dir_all(&effects_dir).unwrap();

    fs::write(src.join("mod.wj"), "pub mod effects\n").unwrap();

    fs::write(
        effects_dir.join("mod.wj"),
        "pub mod bloom_shader\npub mod glow\n",
    )
    .unwrap();

    // Shader file inside "effects/" (not "shaders/")
    fs::write(
        effects_dir.join("bloom_shader.wj"),
        r#"
@fragment
pub fn fs_bloom(@location(0) uv: vec2<float>) -> vec4<float> {
    vec4(1.0, 1.0, 1.0, 1.0)
}
"#,
    )
    .unwrap();

    // Regular code in same directory
    fs::write(
        effects_dir.join("glow.wj"),
        r#"
pub struct GlowEffect {
    intensity: float,
    radius: float,
}
"#,
    )
    .unwrap();

    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--output",
            build.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "Build should succeed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // glow.rs should exist (regular code)
    let effects_build = build.join("effects");
    assert!(
        effects_build.join("glow.rs").exists(),
        "glow.rs should be generated"
    );
}

#[test]
fn test_no_hardcoded_directory_names_in_file_discovery() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let shaders_dir = src.join("shaders");
    let build = dir.path().join("build");
    fs::create_dir_all(&shaders_dir).unwrap();

    fs::write(src.join("mod.wj"), "pub mod shaders\n").unwrap();

    // Regular (non-shader) code in a directory called "shaders"
    fs::write(shaders_dir.join("mod.wj"), "pub mod config\n").unwrap();

    fs::write(
        shaders_dir.join("config.wj"),
        r#"
pub struct ShaderConfig {
    quality: int,
    debug_mode: bool,
}
"#,
    )
    .unwrap();

    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--output",
            build.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "Build should succeed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // config.rs should exist - "shaders" directory should NOT be skipped
    // just because of its name
    let shaders_build = build.join("shaders");
    assert!(
        shaders_build.join("config.rs").exists(),
        "config.rs should be generated even though it's in a 'shaders' directory. \
         Directory name should not affect compilation."
    );
}
