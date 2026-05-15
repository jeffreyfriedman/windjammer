#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

use std::path::PathBuf;

fn game_shaders_available() -> bool {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammer-game/windjammer-game-core/shaders")
        .exists()
}

fn transpile_shader(filename: &str) -> Result<String, String> {
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("windjammer-game")
        .join("windjammer-game-core")
        .join("shaders");
    let path = base_dir.join(filename);
    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", filename, e))?;
    windjammer::wjsl::transpile_wjsl_with_includes(&source, &base_dir).map_err(|e| e.to_string())
}

#[test]
fn test_bloom_extract_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("bloom_extract.wjsl").expect("Should transpile");
    assert!(wgsl.contains("fn main"), "Should have entry point");
}

#[test]
fn test_bloom_extract_has_threshold() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("bloom_extract.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("threshold"),
        "Should use threshold for bright pixel selection"
    );
}

#[test]
fn test_bloom_extract_luminance() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("bloom_extract.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("0.2126") || wgsl.contains("luminance"),
        "Should compute perceptual luminance"
    );
}

#[test]
fn test_bloom_blur_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("bloom_blur.wjsl").expect("Should transpile");
    assert!(wgsl.contains("fn main"), "Should have entry point");
}

#[test]
fn test_bloom_blur_uses_gaussian() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("bloom_blur.wjsl").expect("Should transpile");
    assert!(wgsl.contains("exp("), "Should use Gaussian weights");
}

#[test]
fn test_bloom_blur_reads_input_per_tap() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("bloom_blur.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("blur_input[pidx]"),
        "Should use direct blur_input reads per Gaussian tap (no workgroup scratch)"
    );
}

#[test]
fn test_bloom_combine_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("bloom_combine.wjsl").expect("Should transpile");
    assert!(wgsl.contains("fn main"), "Should have entry point");
}

#[test]
fn test_bloom_combine_strength() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile_shader("bloom_combine.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("bloom_strength"),
        "Should have controllable bloom intensity"
    );
}
