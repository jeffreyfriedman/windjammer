/// TDD Test: Test library should use the correct lib name (not windjammer-app or *_testlib)
///
/// THE WINDJAMMER WAY: Test library name must match project lib name
/// so that test imports like `use windjammer_game_core::*` work correctly.
///
/// Bug: Tests were failing with E0433: unresolved module windjammer_game_core
/// Root Cause: Test library was using wrong names (windjammer-app or *_testlib)
/// Fix: Use the actual project lib name from Cargo.toml
use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_library_uses_correct_lib_name() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_test_lib_name_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // Create a minimal Cargo.toml with lib name
    let cargo_toml = r#"[package]
name = "my-game-core"
version = "0.1.0"
edition = "2021"

[lib]
name = "my_game_core"
path = "src/lib.rs"
"#;
    fs::write(test_dir.join("Cargo.toml"), cargo_toml).unwrap();

    // Create src_wj directory with a simple module
    let src_wj_dir = test_dir.join("src_wj");
    fs::create_dir_all(&src_wj_dir).unwrap();

    // Create a simple Windjammer file
    let wj_content = r#"
struct Player {
    name: String,
    health: i32
}

impl Player {
    fn new(name: String) -> Player {
        Player { name, health: 100 }
    }
}
"#;
    fs::write(src_wj_dir.join("player.wj"), wj_content).unwrap();

    // Create mod.wj to make it a library
    fs::write(src_wj_dir.join("mod.wj"), "pub use player::Player;").unwrap();

    // Create tests_wj directory
    let tests_wj_dir = test_dir.join("tests_wj");
    fs::create_dir_all(&tests_wj_dir).unwrap();

    // Create a test file that imports from the library
    let test_content = r#"
use my_game_core::Player

@test
fn test_player_creation() {
    let player = Player::new("Hero".to_string())
    assert_eq(player.health, 100)
}
"#;
    fs::write(tests_wj_dir.join("player_test.wj"), test_content).unwrap();

    // Run wj test
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("test")
        .output()
        .expect("Failed to run wj test");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Assert that tests pass (no E0433 errors)
    assert!(
        !stderr.contains("E0433") && !stderr.contains("unresolved module or unlinked crate"),
        "Test library should use correct lib name (my_game_core), not windjammer-app or my_game_core_testlib.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );

    // Verify the test actually ran
    assert!(
        output.status.success(),
        "Tests should pass with correct lib name.\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );
}
