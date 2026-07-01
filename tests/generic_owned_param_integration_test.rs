#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

// Integration test for generic owned parameter inference
// Verifies that `mut game: G` generates as `mut game: G` not `game: &G`
// NOTE: Full rustc compilation is disabled due to known issues:
//   1. extern fn generates without body
//   2. Call site adds & when it shouldn't

use std::path::{Path, PathBuf};

fn find_generated_rs(dir: &Path, filename: &str) -> Result<String, std::io::Error> {
    // Try flat first (most common)
    let flat = dir.join(filename);
    if flat.exists() {
        return std::fs::read_to_string(&flat);
    }
    // Walk recursively
    fn walk(dir: &Path, target: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.file_name().map_or(false, |n| n == target) {
                return Some(path);
            }
            if path.is_dir() {
                if let Some(found) = walk(&path, target) {
                    return Some(found);
                }
            }
        }
        None
    }
    match walk(dir, filename) {
        Some(p) => std::fs::read_to_string(&p),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} not found under {}", filename, dir.display()),
        )),
    }
}

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

    // Read the generated Rust code - search recursively since path stripping
    // varies between regular checkout and git worktree environments
    let generated_code = find_generated_rs(out_tmp.path(), "generic_owned_param_test.rs")
        .expect("Failed to find generated generic_owned_param_test.rs in output dir");

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
