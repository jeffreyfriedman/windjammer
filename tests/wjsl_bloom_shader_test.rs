use std::path::PathBuf;

fn transpile_shader(filename: &str) -> Result<String, String> {
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("windjammer-game")
        .join("windjammer-game-core")
        .join("shaders");
    let path = base_dir.join(filename);
    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", filename, e))?;
    windjammer::wjsl::transpile_wjsl_with_includes(&source, &base_dir)
        .map_err(|e| e.to_string())
}

#[test]
fn test_bloom_extract_transpiles() {
    let wgsl = transpile_shader("bloom_extract.wjsl").expect("Should transpile");
    assert!(wgsl.contains("fn main"), "Should have entry point");
}

#[test]
fn test_bloom_extract_has_threshold() {
    let wgsl = transpile_shader("bloom_extract.wjsl").expect("Should transpile");
    assert!(wgsl.contains("threshold"), "Should use threshold for bright pixel selection");
}

#[test]
fn test_bloom_extract_luminance() {
    let wgsl = transpile_shader("bloom_extract.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("0.2126") || wgsl.contains("luminance"),
        "Should compute perceptual luminance"
    );
}

#[test]
fn test_bloom_blur_transpiles() {
    let wgsl = transpile_shader("bloom_blur.wjsl").expect("Should transpile");
    assert!(wgsl.contains("fn main"), "Should have entry point");
}

#[test]
fn test_bloom_blur_uses_gaussian() {
    let wgsl = transpile_shader("bloom_blur.wjsl").expect("Should transpile");
    assert!(wgsl.contains("exp("), "Should use Gaussian weights");
}

#[test]
fn test_bloom_blur_uses_shared_memory() {
    let wgsl = transpile_shader("bloom_blur.wjsl").expect("Should transpile");
    assert!(wgsl.contains("var<workgroup>"), "Should use shared memory for efficiency");
}

#[test]
fn test_bloom_combine_transpiles() {
    let wgsl = transpile_shader("bloom_combine.wjsl").expect("Should transpile");
    assert!(wgsl.contains("fn main"), "Should have entry point");
}

#[test]
fn test_bloom_combine_strength() {
    let wgsl = transpile_shader("bloom_combine.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("bloom_strength"),
        "Should have controllable bloom intensity"
    );
}
