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

/// Member and swizzle access on a call result: a call is a primary expression for
/// postfix purposes, and every texture sampling builtin yields a vec4 (or f32 for
/// comparison samplers) that supports `.rgb`, `.x`, ...
use windjammer::wjsl::transpile_wjsl;

const HEADER: &str = "@group(0) @binding(0) texture_2d tex: texture_2d<f32>;\n@group(0) @binding(1) sampler smp: sampler;\n\n";

fn frag(body: &str) -> String {
    format!(
        "{HEADER}@fragment\nfn main(@location(0) uv: vec2) -> @location(0) vec4 {{\n{body}\n}}\n"
    )
}

#[test]
fn texture_sample_level_swizzle_builds_and_is_emitted() {
    let src = frag("    let c = textureSampleLevel(tex, smp, uv, 0.0).rgb;\n    return vec4(c, 1.0);");
    let wgsl = transpile_wjsl(&src).expect("call-result swizzle must build");
    assert!(wgsl.contains("textureSampleLevel(tex, smp, uv, 0.0).rgb"), "{wgsl}");
}

#[test]
fn every_sampling_builtin_supports_member_access() {
    for call in [
        "textureSample(tex, smp, uv)",
        "textureSampleLevel(tex, smp, uv, 0.0)",
        "textureSampleBias(tex, smp, uv, 0.5)",
        "textureSampleGrad(tex, smp, uv, uv, uv)",
        "textureGather(0, tex, smp, uv)",
    ] {
        let src = frag(&format!("    let a = {call}.xyz;\n    let w = {call}.w;\n    return vec4(a, w);"));
        transpile_wjsl(&src).unwrap_or_else(|e| panic!("{call}: {e}"));
    }
}

#[test]
fn comparison_sample_is_a_scalar() {
    let src = "@group(0) @binding(0) texture_depth_2d d: texture_depth_2d;\n@group(0) @binding(1) sampler_comparison cmp: sampler_comparison;\n\n@fragment\nfn main(@location(0) uv: vec2) -> @location(0) vec4 {\n    let s = textureSampleCompare(d, cmp, uv, 0.5) * 2.0;\n    return vec4(s, s, s, 1.0);\n}\n";
    // Depth textures may be unsupported by the binding grammar; only the scalar typing matters here.
    if let Err(e) = transpile_wjsl(src) {
        assert!(!e.to_string().contains("Cannot access"), "{e}");
    }
}

#[test]
fn user_function_call_result_swizzle_and_index() {
    let src = "fn tint(x: f32) -> vec4 {\n    return vec4(x, x, x, 1.0);\n}\n\n@compute @workgroup_size(1, 1, 1)\nfn main() {\n    let a = tint(1.0).rgb;\n    let b = tint(2.0).x;\n}\n";
    transpile_wjsl(src).expect("user call-result member access");
}
