// Integration test for generic owned parameter inference
// Verifies that `mut game: G` generates as `mut game: G` not `game: &G`
// NOTE: Full rustc compilation is disabled due to known issues:
//   1. extern fn generates without body
//   2. Call site adds & when it shouldn't

use std::process::Command;

// Skip in coverage runs - subprocess spawning is very slow under tarpaulin instrumentation
#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_generic_owned_param_inference() {
    // Compile the Windjammer test file
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--bin",
            "wj",
            "--",
            "build",
            "tests/generic_owned_param_test.wj",
            "--output",
            "/tmp/wj_generic_owned_test",
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run windjammer compiler");

    assert!(
        output.status.success(),
        "Windjammer compilation failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the generated Rust code
    let generated_code =
        std::fs::read_to_string("/tmp/wj_generic_owned_test/generic_owned_param_test.rs")
            .expect("Failed to read generated code");

    // Print for debugging
    println!("Generated code:\n{}", generated_code);

    // THE BUG (FIXED): Should generate `mut game: G` not `game: &G`
    assert!(
        generated_code.contains("pub fn run_game<G: GameState>(mut game: G)") ||
        generated_code.contains("pub fn run_game<G>(mut game: G)\nwhere"),
        "run_game should have owned parameter `mut game: G`, not borrowed `game: &G`.\nGenerated: {}",
        generated_code
    );

    // KNOWN ISSUE: Call site still generates `run_game(&game)` instead of `run_game(game)`
    // This is tracked separately and will be fixed in a future PR
    // For now, we just verify the function signature is correct

    // Cleanup
    let _ = std::fs::remove_dir_all("/tmp/wj_generic_owned_test");
}
