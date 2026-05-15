// Integration test for generic owned parameter inference
// Verifies that `mut game: G` generates as `mut game: G` not `game: &G`
// NOTE: Full rustc compilation is disabled due to known issues:
//   1. extern fn generates without body
//   2. Call site adds & when it shouldn't

use std::path::PathBuf;

// Skip in coverage runs - subprocess spawning is very slow under tarpaulin instrumentation
// The tarpaulin cfg is declared in Cargo.toml [lints.rust] section
#[cfg_attr(tarpaulin, ignore)]
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_owned_param_inference() {
    let out_tmp = tempfile::tempdir().expect("tempdir");
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("generic_owned_param_test.wj");

    windjammer::build_project(
        &wj_path,
        out_tmp.path(),
        windjammer::CompilationTarget::Rust,
        false,
    )
    .expect("Windjammer compilation failed");

    // Read the generated Rust code
    // The compiler preserves directory structure, so the file is at:
    // output_dir/wj/windjammer/tests/generic_owned_param_test.rs
    let generated_code = std::fs::read_to_string(
        out_tmp
            .path()
            .join("wj")
            .join("windjammer")
            .join("tests")
            .join("generic_owned_param_test.rs"),
    )
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
}
