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

// TDD TEST: Struct with Vec field should NOT be Copy
//
// BUG: GameState struct with Vec<(string, bool)> field was being detected as Copy,
// causing E0382 errors when passing game_state to multiple functions.
//
// ROOT CAUSE: Copy detection logic was incorrectly marking structs with Vec fields as Copy.
//
// FIX: Ensure Vec fields prevent struct from being Copy.

use std::fs;
use std::process::Command;

#[test]
fn test_struct_with_vec_field_not_copy() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

    let wj_code = r#"
pub struct PlayerState {
    pub name: string,
    pub health: i32,
}

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState { name: "Player".to_string(), health: 100 }
    }
}

pub struct GameState {
    pub player: PlayerState,
    pub flags: Vec<(string, bool)>,
}

impl GameState {
    pub fn new() -> GameState {
        GameState {
            player: PlayerState::new(),
            flags: Vec::new(),
        }
    }
}

pub fn check_flag(game_state: GameState, flag_id: string) -> bool {
    for (id, value) in game_state.flags {
        if id == flag_id {
            return value
        }
    }
    false
}

pub fn use_twice(game_state: GameState) -> bool {
    // TDD: This should compile without E0382 because game_state should be &GameState
    let first = check_flag(game_state, "test1");
    let second = check_flag(game_state, "test2");
    first && second
}
"#;

    let wj_file = temp_dir.path().join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            temp_dir.path().to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("=== WJ COMPILE OUTPUT ===");
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    let src_dir = temp_dir.path().join("src");
    let rs_file = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        temp_dir.path().join("test.rs")
    };
    let rust_code = fs::read_to_string(&rs_file).expect("Failed to read generated Rust file");

    println!("=== GENERATED RUST ===");
    println!("{}", rust_code);

    assert!(
        rust_code.contains("fn check_flag(game_state: &GameState"),
        "check_flag should take &GameState (borrowed), not GameState (owned)\n\
         This means GameState was wrongly detected as Copy!"
    );

    assert!(
        rust_code.contains("fn use_twice(game_state: &GameState"),
        "use_twice should take &GameState (borrowed), not GameState (owned)"
    );

    let rlib_output = temp_dir.path().join("output.rlib");
    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&rs_file)
        .output()
        .expect("Failed to run rustc");

    let rustc_stderr = String::from_utf8_lossy(&rustc_output.stderr);
    println!("=== RUSTC OUTPUT ===");
    println!("{}", rustc_stderr);

    assert!(
        !rustc_stderr.contains("E0382"),
        "Generated Rust has E0382 (use of moved value)! GameState was wrongly marked as Copy.\n{}",
        rustc_stderr
    );

    assert!(
        rustc_output.status.success(),
        "Rustc compilation failed:\n{}",
        rustc_stderr
    );
}
