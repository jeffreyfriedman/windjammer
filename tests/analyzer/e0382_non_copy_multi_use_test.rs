// TDD TEST: E0382 fix - Non-Copy struct used multiple times should infer Borrowed
//
// ROOT CAUSE: GameState has Vec fields (NOT Copy), but metadata from other files
// says it's Copy. Even after fixing metadata loading, ownership inference still
// generates Owned signatures.
//
// EXPECTED: Parameters should be &GameState (Borrowed) when used multiple times

use std::fs;
use std::process::Command;

#[test]
fn test_non_copy_struct_multi_use_infers_borrowed() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();

    let wj_code = r#"
pub struct PlayerState {
    pub health: i32,
}

pub struct GameState {
    pub player: PlayerState,
    pub flags: Vec<(string, bool)>,  // Vec makes it non-Copy!
}

pub fn check_flag(game_state: GameState, flag_id: string) -> bool {
    for (id, value) in game_state.flags {
        if id == flag_id {
            return value
        }
    }
    false
}

pub fn is_available(game_state: GameState) -> bool {
    // Using game_state TWICE - should infer &GameState (Borrowed)
    let first = check_flag(game_state, "test1");
    let second = check_flag(game_state, "test2");
    first && second
}
"#;

    let wj_file = temp_dir.path().join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    let wj_bin = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_bin)
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("=== WJ COMPILE OUTPUT ===");
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    let rs_file = out_dir.join("test.rs");

    // Debug: list what's in temp if output missing
    if !rs_file.exists() {
        println!("=== TEMP DIR CONTENTS ===");
        for entry in fs::read_dir(temp_dir.path()).unwrap() {
            let entry = entry.unwrap();
            println!("  {:?}", entry.path());
        }
        if let Ok(entries) = fs::read_dir(&out_dir) {
            println!("=== OUT DIR CONTENTS ===");
            for entry in entries {
                let entry = entry.unwrap();
                println!("  {:?}", entry.path());
            }
        }
    }

    let generated = fs::read_to_string(&rs_file).expect("Failed to read generated Rust file");

    println!("=== GENERATED RUST ===");
    println!("{}", generated);

    // TDD ASSERTION 1: check_flag should take &GameState (borrowed), not GameState (owned)
    // If Copy detection is wrong, game_state will be owned and cause E0382
    assert!(
        generated.contains("fn check_flag(game_state: &GameState"),
        "check_flag should take &GameState (borrowed), not GameState (owned)\n\
         This means GameState was wrongly detected as Copy!"
    );

    // TDD ASSERTION 2: is_available should also take &GameState
    assert!(
        generated.contains("fn is_available(game_state: &GameState"),
        "is_available should take &GameState (borrowed), not GameState (owned)"
    );

    // TDD ASSERTION 3: The critical test - verify no E0382 in generated code
    // Note: There may be other errors (E0308 for tuple destructuring, string literals),
    // but E0382 specifically tests if GameState was wrongly marked as Copy.
    let rustc_out = temp_dir.path().join("rustc_out");
    fs::create_dir_all(&rustc_out).unwrap();
    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            rs_file.to_str().unwrap(),
            "--out-dir",
            rustc_out.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let rustc_stderr = String::from_utf8_lossy(&rustc_output.stderr);
    println!("=== RUSTC OUTPUT ===");
    println!("{}", rustc_stderr);

    // THE CRITICAL ASSERTION: No E0382 (use of moved value)
    // E0382 would appear if game_state was passed by value (owned) instead of by reference.
    // This confirms the conservative Copy detection fix is working!
    assert!(
        !rustc_stderr.contains("E0382"),
        "FAIL: Generated Rust has E0382 (use of moved value)!\n\
         This means GameState was wrongly marked as Copy.\n\
         The conservative Copy detection fix is NOT working.\n{}",
        rustc_stderr
    );

    println!("\n✅ SUCCESS: No E0382 errors! GameState correctly inferred as non-Copy.");
    println!("✅ Ownership inference generated &GameState (Borrowed) signatures.");
}
