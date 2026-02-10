// TDD Test: Build System Should Propagate FFI Dependencies
// Bug: When wj build compiles a project with FFI dependencies (wgpu, bytemuck, etc.),
//      the generated build/Cargo.toml doesn't include those dependencies.
// Root Cause: create_cargo_toml_with_deps() doesn't read source project's Cargo.toml
// Fix: Copy dependencies from source Cargo.toml to generated Cargo.toml
// Impact: Enables actual game compilation and rendering

use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_build_system_propagates_ffi_dependencies() {
    // Create a temp directory for our test project
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("test_game");
    fs::create_dir_all(&project_dir).unwrap();

    // Create a simple Windjammer file that uses game engine
    let game_file = project_dir.join("simple_game.wj");
    fs::write(
        &game_file,
        r#"// Simple game that uses FFI
use windjammer_game_core::prelude::*

fn main() {
    println!("Game!")
}
"#,
    )
    .unwrap();

    // Create a Cargo.toml with FFI dependencies (simulating windjammer-game-core)
    let cargo_toml = project_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "test-game"
version = "0.1.0"
edition = "2021"

[dependencies]
# These FFI dependencies should be copied to build/Cargo.toml
wgpu = "0.19"
winit = "0.29"
pollster = "0.3"
bytemuck = { version = "1.14", features = ["derive"] }
rapier3d = "0.17"
"#,
    )
    .unwrap();

    // Create a minimal ffi/ directory to signal this project has FFI
    let ffi_dir = project_dir.join("ffi");
    fs::create_dir_all(&ffi_dir).unwrap();
    fs::write(
        ffi_dir.join("mod.rs"),
        "// FFI module that uses wgpu\npub fn init() {}",
    )
    .unwrap();

    // Run wj build (with --no-cargo to avoid cargo build)
    // Use the locally built wj binary, not the one from PATH
    let wj_binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wj");

    println!("Using wj binary at: {:?}", wj_binary);

    // Explicitly specify output directory
    let build_dir = project_dir.join("build");
    let output = std::process::Command::new(&wj_binary)
        .args([
            "build",
            game_file.to_str().unwrap(),
            "--output",
            build_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(&project_dir)
        .output()
        .expect("Failed to run wj build");

    // Print output for debugging
    println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));

    // Verify the command succeeded
    if !output.status.success() {
        panic!("wj build failed");
    }

    // Read the generated Cargo.toml
    let generated_cargo_toml = build_dir.join("Cargo.toml");
    assert!(
        generated_cargo_toml.exists(),
        "build/Cargo.toml should be generated"
    );

    let generated_content = fs::read_to_string(&generated_cargo_toml).unwrap();
    println!("Generated Cargo.toml:\n{}", generated_content);

    // THE TEST: Verify FFI dependencies were copied
    assert!(
        generated_content.contains("wgpu"),
        "build/Cargo.toml should include wgpu dependency"
    );
    assert!(
        generated_content.contains("bytemuck"),
        "build/Cargo.toml should include bytemuck dependency"
    );
    assert!(
        generated_content.contains("pollster"),
        "build/Cargo.toml should include pollster dependency"
    );
    assert!(
        generated_content.contains("rapier3d"),
        "build/Cargo.toml should include rapier3d dependency"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_build_system_without_ffi_still_works() {
    // Create a temp directory for a simple project WITHOUT FFI
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("simple_project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create a simple Windjammer file without FFI
    let simple_file = project_dir.join("hello.wj");
    fs::write(
        &simple_file,
        r#"fn main() {
    println!("Hello!")
}
"#,
    )
    .unwrap();

    // Run wj build with locally built binary
    let wj_binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wj");

    let build_dir = project_dir.join("build");
    let output = std::process::Command::new(&wj_binary)
        .args([
            "build",
            simple_file.to_str().unwrap(),
            "--output",
            build_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(&project_dir)
        .output()
        .expect("Failed to run wj build");

    // Should still succeed
    assert!(
        output.status.success(),
        "wj build should work for non-FFI projects"
    );

    // Generated Cargo.toml should exist but not have FFI deps
    let generated_cargo_toml = build_dir.join("Cargo.toml");
    assert!(generated_cargo_toml.exists());

    let generated_content = fs::read_to_string(&generated_cargo_toml).unwrap();

    // Should NOT contain FFI deps (since source project didn't have them)
    assert!(
        !generated_content.contains("wgpu") || generated_content.contains("windjammer-game-core"),
        "Non-FFI project shouldn't randomly get wgpu"
    );
}
