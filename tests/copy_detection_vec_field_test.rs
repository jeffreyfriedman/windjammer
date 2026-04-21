// TDD TEST: Struct with Vec field should NOT be Copy
//
// BUG: GameState struct with Vec<(string, bool)> field was being detected as Copy,
// causing E0382 errors when passing game_state to multiple functions.
//
// ROOT CAUSE: Copy detection logic was incorrectly marking structs with Vec fields as Copy.
//
// FIX: Ensure Vec fields prevent struct from being Copy.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_struct_with_vec_field_not_copy() {
    let temp_dir = std::env::temp_dir().join("wj_test_copy_vec");
    fs::create_dir_all(&temp_dir).unwrap();

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

    let wj_file = temp_dir.join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    // Find wj compiler (use local build if available)
    let wj_bin = if PathBuf::from("./target/release/wj").exists() {
        "./target/release/wj"
    } else {
        "wj"
    };

    // Compile with wj
    let output = Command::new(wj_bin)
        .args(["build", wj_file.to_str().unwrap(), "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("=== WJ COMPILE OUTPUT ===");
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    // Read generated Rust code
    let rs_file = temp_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rs_file).expect("Failed to read generated Rust file");

    println!("=== GENERATED RUST ===");
    println!("{}", rust_code);

    // TDD ASSERTION 1: GameState should take &GameState, not GameState (owned)
    // If Copy detection is wrong, game_state will be owned and cause E0382
    assert!(
        rust_code.contains("fn check_flag(game_state: &GameState"),
        "check_flag should take &GameState (borrowed), not GameState (owned)\n\
         This means GameState was wrongly detected as Copy!"
    );

    assert!(
        rust_code.contains("fn use_twice(game_state: &GameState"),
        "use_twice should take &GameState (borrowed), not GameState (owned)"
    );

    // TDD ASSERTION 2: Verify rustc compiles without E0382
    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            rs_file.to_str().unwrap(),
            "--out-dir",
            temp_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let rustc_stderr = String::from_utf8_lossy(&rustc_output.stderr);
    println!("=== RUSTC OUTPUT ===");
    println!("{}", rustc_stderr);

    // Check for E0382 (use of moved value)
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

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
