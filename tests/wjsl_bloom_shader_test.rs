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

#[path = "common/wjsl_shader_fixtures.rs"]
mod wjsl_shader_fixtures;


#[test]
fn test_bloom_extract_transpiles() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("bloom_extract.wjsl").expect("Should transpile");
    assert!(wgsl.contains("fn main"), "Should have entry point");
}

#[test]
fn test_bloom_extract_has_threshold() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("bloom_extract.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("threshold"),
        "Should use threshold for bright pixel selection"
    );
}

#[test]
fn test_bloom_extract_luminance() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("bloom_extract.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("0.2126") || wgsl.contains("luminance"),
        "Should compute perceptual luminance"
    );
}

#[test]
fn test_bloom_blur_transpiles() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("bloom_blur.wjsl").expect("Should transpile");
    assert!(wgsl.contains("fn main"), "Should have entry point");
}

#[test]
fn test_bloom_blur_uses_gaussian() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("bloom_blur.wjsl").expect("Should transpile");
    assert!(wgsl.contains("exp("), "Should use Gaussian weights");
}

#[test]
fn test_bloom_blur_reads_input_per_tap() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("bloom_blur.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("blur_input[pidx]"),
        "Should use direct blur_input reads per Gaussian tap (no workgroup scratch)"
    );
}

#[test]
fn test_bloom_combine_transpiles() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("bloom_combine.wjsl").expect("Should transpile");
    assert!(wgsl.contains("fn main"), "Should have entry point");
}

#[test]
fn test_bloom_combine_strength() {
    let wgsl = wjsl_shader_fixtures::transpile_fixture_shader("bloom_combine.wjsl").expect("Should transpile");
    assert!(
        wgsl.contains("bloom_strength"),
        "Should have controllable bloom intensity"
    );
}
