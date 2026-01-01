/// TDD Test: Same-name module files should NOT become 0 bytes during regeneration
///
/// BUG: Files like game_loop/game_loop.wj compile correctly in the first pass,
/// but during trait inference regeneration, they become 0 bytes.
///
/// EXPECTED: Regeneration should preserve file content, not create empty files.
use std::process::Command;

#[test]
fn test_same_name_module_regeneration_not_empty() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_wj = temp_dir.path().join("src_wj");
    std::fs::create_dir_all(&src_wj).unwrap();

    // Create game_loop subdirectory with same-name file
    let game_loop_dir = src_wj.join("game_loop");
    std::fs::create_dir_all(&game_loop_dir).unwrap();

    // game_loop/mod.wj
    std::fs::write(
        game_loop_dir.join("mod.wj"),
        r#"
pub mod game_loop;
"#,
    )
    .unwrap();

    // game_loop/game_loop.wj (same name as parent directory)
    std::fs::write(
        game_loop_dir.join("game_loop.wj"),
        r#"
pub trait GameLoop {
    fn init(&mut self) {
        // Default implementation
    }
}
"#,
    )
    .unwrap();

    // Create a game module that uses the trait
    std::fs::write(
        src_wj.join("game.wj"),
        r#"
use crate::game_loop::game_loop::GameLoop

pub struct Game {
    pub score: i32,
}

impl GameLoop for Game {
    fn init(&mut self) {
        self.score = 0;
    }
}
"#,
    )
    .unwrap();

    // Root mod.wj
    std::fs::write(
        src_wj.join("mod.wj"),
        r#"
pub mod game_loop;
pub mod game;
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("out");
    let compile_result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(src_wj.join("mod.wj"))
        .arg("--output")
        .arg(&output_dir)
        .arg("--library")
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute compiler");

    if !compile_result.status.success() {
        eprintln!(
            "STDOUT:\n{}",
            String::from_utf8_lossy(&compile_result.stdout)
        );
        eprintln!(
            "STDERR:\n{}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        panic!("Compiler failed");
    }

    // Check the generated game_loop/game_loop.rs file
    let game_loop_rs = output_dir.join("game_loop/game_loop.rs");
    assert!(game_loop_rs.exists(), "game_loop/game_loop.rs should exist");

    let metadata = std::fs::metadata(&game_loop_rs).expect("Failed to get metadata");
    let file_size = metadata.len();

    println!("game_loop/game_loop.rs size: {} bytes", file_size);

    // CRITICAL: File should NOT be empty
    assert!(
        file_size > 0,
        "game_loop/game_loop.rs should NOT be 0 bytes! File is empty after regeneration."
    );

    // Verify it actually contains the trait
    let content = std::fs::read_to_string(&game_loop_rs).expect("Failed to read game_loop.rs");

    println!(
        "=== game_loop/game_loop.rs content ===\n{}\n========================",
        content
    );

    assert!(
        content.contains("trait GameLoop"),
        "File should contain the GameLoop trait definition!"
    );
}
