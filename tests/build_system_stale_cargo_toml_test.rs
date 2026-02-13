// TDD Test: Build System Should Regenerate Cargo.toml Even When Stale One Exists
// Bug: When a stale Cargo.toml exists in the output directory from a previous build,
//      the is_component_project flag is set to true prematurely (on the first file
//      with no stdlib imports), preventing Cargo.toml regeneration.
// Root Cause: is_component_project detection runs inside the per-file loop and triggers
//      on ANY file with empty imports when Cargo.toml exists, rather than checking
//      the aggregate of ALL files.
// Fix: Move is_component_project detection to after the compilation loop, checking
//      aggregated all_stdlib_modules and all_external_crates.
// Impact: Stale Cargo.toml with incorrect [[bin]] targets causes E0601 and E0433 errors

use std::fs;
use tempfile::TempDir;

/// Helper: get the wj binary path from the test binary location
fn get_wj_binary() -> std::path::PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wj")
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_cargo_toml_regenerated_when_stale_one_exists() {
    // Create a multi-file project
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src_wj");
    let build_dir = temp_dir.path().join("build");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&build_dir).unwrap();

    // Create mod.wj with two modules
    fs::write(
        src_dir.join("mod.wj"),
        r#"
mod math
mod game
"#,
    )
    .unwrap();

    // Create math.wj (no stdlib imports)
    fs::write(
        src_dir.join("math.wj"),
        r#"
pub struct Vec2 {
    pub x: float,
    pub y: float,
}

impl Vec2 {
    pub fn new(x: float, y: float) -> Vec2 {
        Vec2 { x: x, y: y }
    }

    pub fn length(self) -> float {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
"#,
    )
    .unwrap();

    // Create game.wj (no stdlib imports)
    fs::write(
        src_dir.join("game.wj"),
        r#"
pub struct Game {
    pub name: String,
    pub running: bool,
}

impl Game {
    pub fn new(name: String) -> Game {
        Game { name: name, running: false }
    }
}
"#,
    )
    .unwrap();

    // Plant a STALE Cargo.toml with a broken [[bin]] target
    // This simulates the scenario where a previous build left a stale Cargo.toml
    fs::write(
        build_dir.join("Cargo.toml"),
        r#"[package]
name = "windjammer-app"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
smallvec = "1.13"

[[bin]]
name = "stale_binary"
path = "stale_binary.rs"

[profile.release]
opt-level = 3
"#,
    )
    .unwrap();

    // Also plant a stale .rs file (simulates leftover from previous build)
    fs::write(
        build_dir.join("stale_binary.rs"),
        "// This is a stale file from a previous build\nfn stale() {}\n",
    )
    .unwrap();

    // Run wj build
    let wj_binary = get_wj_binary();
    let output = std::process::Command::new(&wj_binary)
        .args([
            "build",
            src_dir.join("mod.wj").to_str().unwrap(),
            "--output",
            build_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj build");

    println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));

    assert!(
        output.status.success(),
        "wj build should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // THE KEY ASSERTION: Cargo.toml should be regenerated with [lib], not the stale [[bin]]
    let cargo_toml_content =
        fs::read_to_string(build_dir.join("Cargo.toml")).expect("Cargo.toml should exist");
    println!("Generated Cargo.toml:\n{}", cargo_toml_content);

    // Should have [lib] section (because lib.rs was generated for multi-file project)
    assert!(
        cargo_toml_content.contains("[lib]"),
        "Cargo.toml should have [lib] section (lib.rs exists for multi-file project)"
    );

    // Should NOT have the stale [[bin]] target
    assert!(
        !cargo_toml_content.contains("stale_binary"),
        "Cargo.toml should NOT contain stale [[bin]] target from previous build"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_lib_rs_generated_for_multi_file_project() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src_wj");
    let build_dir = temp_dir.path().join("build");
    fs::create_dir_all(&src_dir).unwrap();

    // Create mod.wj with a module
    fs::write(
        src_dir.join("mod.wj"),
        r#"
mod utils
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("utils.wj"),
        r#"
pub fn add(a: int, b: int) -> int {
    a + b
}
"#,
    )
    .unwrap();

    let wj_binary = get_wj_binary();
    let output = std::process::Command::new(&wj_binary)
        .args([
            "build",
            src_dir.join("mod.wj").to_str().unwrap(),
            "--output",
            build_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj build");

    println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "wj build should succeed");

    // lib.rs should exist
    assert!(
        build_dir.join("lib.rs").exists(),
        "lib.rs should be generated for multi-file project"
    );

    // Cargo.toml should have [lib] section
    let cargo_toml = fs::read_to_string(build_dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo_toml.contains("[lib]"),
        "Cargo.toml should have [lib] section when lib.rs exists"
    );
}
