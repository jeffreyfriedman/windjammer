#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

// TDD Test: Parameter used multiple times in loop should be inferred as borrowed
// Reproduces E0382 errors in dialog.wj lines 92-97, 129-131, 129-146, 169-172
//
// PROBLEM:
// game_state: GameState is passed as owned, but used multiple times in for-loop
// First iteration moves game_state, second iteration fails with E0382
//
// EXAMPLE:
// pub fn is_available(self, game_state: GameState) -> bool {
//     for condition in self.conditions {
//         if !condition.evaluate(game_state) { // MOVES game_state!
//             return false
//         }
//     }
//     true
// }
//
// SOLUTION:
// Analyzer should infer game_state as &GameState because:
// 1. Used in loop (multiple potential uses)
// 2. Passed to method that doesn't consume it

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_param_used_multiple_times_in_loop() {
    let wj_code = r#"
struct GameState {
    player_name: string,  // String makes struct non-Copy!
    player_health: i32,
}

enum Condition {
    HealthAbove(i32),
}

impl Condition {
    pub fn check(self, state: GameState) -> bool {
        match self {
            Condition::HealthAbove(min) => state.player_health >= min,
        }
    }
}

struct DialogNode {
    conditions: Vec<Condition>,
}

impl DialogNode {
    pub fn is_available(self, game_state: GameState) -> bool {
        for condition in self.conditions {
            if !condition.check(game_state) {
                return false
            }
        }
        true
    }
}
"#;

    let temp = TempDir::new().expect("tempdir");
    let wj_file = temp.path().join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--no-cargo")
        .current_dir(temp.path())
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "wj compilation failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }

    let build_rs = temp.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&build_rs).expect("Failed to read generated Rust");

    // ASSERT: game_state should be inferred as &GameState (not owned)
    // because it's used multiple times in the loop
    assert!(
        generated.contains("is_available(&self, game_state: &GameState)")
            || generated.contains("is_available(self, game_state: &GameState)"),
        "game_state should be inferred as &GameState when used multiple times in loop. Generated:\n{}",
        generated
    );

    let output = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(&build_rs)
        .output()
        .unwrap();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Generated code should compile without E0382. Error:\n{}\n\nGenerated:\n{}",
            stderr, generated
        );
    }
}
