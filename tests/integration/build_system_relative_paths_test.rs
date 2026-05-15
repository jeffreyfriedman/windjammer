// TDD Test: Build System Should Convert Relative Paths to Absolute
// Bug: When copying dependencies from source Cargo.toml, relative path dependencies
//      (like windjammer-runtime = { path = "../../windjammer/crates/..." })
//      don't work from build/ directory
// Root Cause: Relative paths are copied as-is, not adjusted for new location
// Fix: Convert relative paths to absolute paths when copying dependencies

use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_build_system_converts_relative_paths_to_absolute() {
    // Create a temp directory structure simulating windjammer-game-core
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create a simple Windjammer file
    let game_file = project_dir.join("game.wj");
    fs::write(
        &game_file,
        r#"fn main() {
    println!("Game!")
}
"#,
    )
    .unwrap();

    // Create a Cargo.toml with RELATIVE path dependency
    let cargo_toml = project_dir.join("Cargo.toml");
    let fake_runtime_path = temp_dir
        .path()
        .join("windjammer")
        .join("crates")
        .join("windjammer-runtime");
    fs::create_dir_all(&fake_runtime_path).unwrap();
    fs::create_dir_all(fake_runtime_path.join("src")).unwrap();

    // Create a proper Cargo.toml with [lib]
    fs::write(
        fake_runtime_path.join("Cargo.toml"),
        r#"[package]
name = "windjammer-runtime"
version = "0.1.0"
edition = "2021"

[lib]
name = "windjammer_runtime"
path = "src/lib.rs"
"#,
    )
    .unwrap();

    // Create a minimal lib.rs
    fs::write(
        fake_runtime_path.join("src/lib.rs"),
        "// Minimal fake runtime\n",
    )
    .unwrap();

    // Write Cargo.toml with relative path (simulating windjammer-game-core scenario)
    fs::write(
        &cargo_toml,
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
# Relative path that works from project_dir but not from project_dir/build
windjammer-runtime = { path = "../windjammer/crates/windjammer-runtime" }
wgpu = "0.19"
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
            game_file.to_str().unwrap(),
            "--output",
            build_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(&project_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
        panic!("wj build failed");
    }

    // Read the generated Cargo.toml
    let generated_cargo_toml = build_dir.join("Cargo.toml");
    assert!(generated_cargo_toml.exists());

    let generated_content = fs::read_to_string(&generated_cargo_toml).unwrap();
    println!("Generated Cargo.toml:\n{}", generated_content);

    // THE TEST: Verify the path was converted to ABSOLUTE
    // It should NOT contain "../" relative paths
    assert!(
        !generated_content.contains("path = \"../"),
        "Generated Cargo.toml should not have relative paths (../)found:\n{}",
        generated_content
    );

    // It should contain an absolute path or a corrected relative path
    // The path should be valid from the build directory
    assert!(
        generated_content.contains("windjammer-runtime"),
        "windjammer-runtime dependency should still be present"
    );

    // Most importantly: Try to actually run cargo check from build dir
    // This will fail if the path is wrong
    let cargo_check = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&build_dir)
        .output()
        .expect("Failed to run cargo metadata");

    if !cargo_check.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_check.stderr);
        eprintln!("Cargo metadata stderr:\n{}", stderr);

        // This is the actual test - cargo should be able to resolve dependencies
        assert!(
            !stderr.contains("failed to load manifest")
                && !stderr.contains("No such file or directory"),
            "Cargo should be able to resolve all dependencies from build/. \
             The relative path was not correctly converted.\nError: {}",
            stderr
        );
    }
}
