/// TDD Test: Cross-file trait inference should preserve explicit &mut self
///
/// BUG: In multi-file projects, when a trait has explicit `fn init(&mut self)`
/// and an implementation in another file doesn't mutate, the analyzer infers `&self`
/// and overrides the explicit trait definition.
///
/// EXPECTED: Explicit ownership in trait definitions should NEVER be overridden,
/// regardless of what implementations do.
use std::process::Command;

#[test]
fn test_cross_file_trait_preserves_explicit_mut() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_wj = temp_dir.path().join("src_wj");
    std::fs::create_dir_all(&src_wj).unwrap();

    // Create game_loop module with trait (explicit &mut self)
    std::fs::write(
        src_wj.join("game_loop.wj"),
        r#"
pub trait GameLoop {
    // EXPLICIT &mut self - should be preserved even if impl doesn't mutate
    fn init(&mut self) {
        // Empty default - doesn't mutate, but signature should be preserved
    }
    
    fn update(&mut self, delta: f32) {
        // Empty default
    }
}
"#,
    )
    .unwrap();

    // Create game module with implementation
    std::fs::write(
        src_wj.join("game.wj"),
        r#"
use crate::game_loop::GameLoop

pub struct Game {
    pub score: i32,
}

impl GameLoop for Game {
    fn init(&mut self) {
        // Impl also doesn't mutate, but should match trait's explicit &mut self
        println!("Game initialized");
    }
    
    fn update(&mut self, delta: f32) {
        self.score += 1;
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

    // Check the generated trait file
    let trait_rust = std::fs::read_to_string(output_dir.join("game_loop.rs"))
        .expect("Failed to read generated trait");

    println!(
        "=== Generated game_loop.rs ===\n{}\n=====================",
        trait_rust
    );

    // CRITICAL: Trait should preserve explicit &mut self
    assert!(
        trait_rust.contains("fn init(&mut self)"),
        "Trait definition should preserve explicit &mut self even if default doesn't mutate!\nGenerated:\n{}",
        trait_rust
    );

    assert!(
        !trait_rust.contains("fn init(&self)"),
        "Trait should NOT have &self when source explicitly declared &mut self!\nGenerated:\n{}",
        trait_rust
    );
}
