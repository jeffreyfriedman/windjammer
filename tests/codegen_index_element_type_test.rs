#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// Test: Indexed access on Vec fields resolves the element type for method signature lookup.
///
/// Bug: `self.enemy_ais[i].update(self.enemies[i], ...)` generated `&self.enemies[i]`
/// instead of `&mut self.enemies[i]` because `infer_type_name` returned "Vec" (the
/// collection type) for `self.enemies[i]` instead of "Enemy" (the element type).
/// The signature lookup then tried `Vec::update` which doesn't exist, so
/// method_signature was None and auto-mut-borrow was skipped.
///
/// Fix: For `Expression::Index { object, .. }`, extract the element type from the
/// collection's declared type (Vec<T> → T, Array<T, N> → T).
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_indexed_vec_field_method_call_uses_element_type() {
    let temp_dir = TempDir::new().unwrap();

    let enemy_source = r#"
pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub alive: bool,
}

impl Enemy {
    pub fn new() -> Enemy {
        Enemy { x: 0.0, y: 0.0, alive: true }
    }
}
"#;

    let ai_source = r#"
use crate::enemy::Enemy

pub struct EnemyAI {
    pub cooldown: f32,
}

impl EnemyAI {
    pub fn new() -> EnemyAI {
        EnemyAI { cooldown: 0.0 }
    }

    pub fn update(self, enemy: Enemy, dt: f32) {
        self.cooldown = self.cooldown - dt
        enemy.x = enemy.x + dt
    }
}
"#;

    let game_source = r#"
use crate::enemy::Enemy
use crate::enemy_ai::EnemyAI

pub struct Game {
    pub enemies: Vec<Enemy>,
    pub enemy_ais: Vec<EnemyAI>,
}

impl Game {
    pub fn tick(self, dt: f32) {
        let mut i: usize = 0
        while i < self.enemies.len() {
            self.enemy_ais[i].update(self.enemies[i], dt)
            i = i + 1
        }
    }
}
"#;

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("enemy.wj"), enemy_source).unwrap();
    fs::write(src_dir.join("enemy_ai.wj"), ai_source).unwrap();
    fs::write(src_dir.join("game.wj"), game_source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("failed to run wj");

    assert!(
        wj_output.status.success(),
        "wj build failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&wj_output.stdout),
        String::from_utf8_lossy(&wj_output.stderr)
    );

    let game_rs = temp_dir.path().join("build").join("game.rs");
    let generated = fs::read_to_string(&game_rs).unwrap_or_else(|_| {
        panic!(
            "generated game.rs not found.\nBuild output:\n{}",
            String::from_utf8_lossy(&wj_output.stdout)
        )
    });

    assert!(
        generated.contains("&mut self.enemies[i]"),
        "Expected `&mut self.enemies[i]` (MutBorrowed param) but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("&self.enemies[i]"),
        "Should NOT have immutable `&self.enemies[i]` when param is MutBorrowed:\n{}",
        generated
    );
}
