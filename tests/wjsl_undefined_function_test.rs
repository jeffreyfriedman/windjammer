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

/// A call to a name that is no WGSL builtin, constructor, struct or user function
/// is a build-time diagnostic (WJ0195), not a late naga failure.
use windjammer::wjsl::builtins::{is_builtin_function, WGSL_BUILTINS};
use windjammer::wjsl::transpile_wjsl;

#[test]
fn undefined_function_call_is_reported_with_name_and_suggestion() {
    let src = r#"
fn lighten(x: f32) -> f32 {
    return x + 1.0;
}

@compute @workgroup_size(1, 1, 1)
fn main() {
    let a = 1.0;
    let b = lighten_it(a);
}
"#;
    let err = transpile_wjsl(src).expect_err("undefined function must not build");
    let msg = err.to_string();
    assert!(msg.contains("WJ0195"), "{msg}");
    assert!(msg.contains("`lighten_it`"), "{msg}");
    assert!(msg.contains("line 9"), "{msg}");
    assert!(msg.contains("did you mean `lighten`"), "{msg}");
}

#[test]
fn misspelled_builtin_suggests_the_builtin() {
    let src = r#"
@compute @workgroup_size(1, 1, 1)
fn main() {
    let a = 1.0;
    let b = normalise(vec3(a, a, a));
}
"#;
    let msg = transpile_wjsl(src).expect_err("undefined").to_string();
    assert!(msg.contains("WJ0195") && msg.contains("did you mean `normalize`"), "{msg}");
}

#[test]
fn undefined_void_call_statement_is_reported() {
    let src = r#"
@compute @workgroup_size(1, 1, 1)
fn main() {
    do_nothing();
}
"#;
    let msg = transpile_wjsl(src).expect_err("undefined").to_string();
    assert!(msg.contains("WJ0195") && msg.contains("`do_nothing`"), "{msg}");
}

#[test]
fn user_void_function_and_struct_constructor_are_not_undefined() {
    let src = r#"
struct P { a: f32, b: f32 }

fn bump() {
    let z = 1.0;
}

@compute @workgroup_size(1, 1, 1)
fn main() {
    bump();
    let p = P(1.0, 2.0);
}
"#;
    transpile_wjsl(src).expect("known callees build");
}

#[test]
fn spec_builtins_are_known() {
    for name in [
        "atomicAdd", "dpdx", "fwidth", "textureStore", "textureSampleLevel", "pack4x8unorm",
        "workgroupBarrier", "countOneBits", "extractBits", "fma", "frexp", "textureDimensions",
        "storageBarrier", "subgroupAdd", "quantizeToF16",
    ] {
        assert!(is_builtin_function(name), "{name} missing from the builtin table");
    }
    assert!(!is_builtin_function("lighten_it"));
    let mut sorted = WGSL_BUILTINS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), WGSL_BUILTINS.len(), "duplicate builtin entries");
}

#[test]
fn builtin_calls_in_a_realistic_shader_build() {
    let src = r#"
@group(0) @binding(0) storage read_write counters: array<atomic<u32>>;

@compute @workgroup_size(1, 1, 1)
fn main() {
    let a = 2.0;
    let b = fma(a, a, a) + fract(a) + abs(a) + saturate(a);
    workgroupBarrier();
}
"#;
    transpile_wjsl(src).expect("builtins are known");
}
