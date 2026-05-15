//! TDD test: @workgroup_size with 2 arguments (x, y) should default z=1
//!
//! Bug: The WJSL parser used `self.peek()` instead of `self.current` to check
//! for the closing RParen after the y component, causing a parse failure when
//! only 2 workgroup dimensions were specified.

#[test]
fn test_workgroup_size_two_args() {
    let source = r#"
@group(0) @binding(0) storage read_write buf: array<f32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    buf[id.x] = 1.0;
}
"#;
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "2-arg workgroup_size failed: {:?}",
        result.err()
    );
    let wgsl = result.unwrap();
    assert!(
        wgsl.contains("@workgroup_size(8, 8, 1)") || wgsl.contains("@workgroup_size(8u, 8u, 1u)"),
        "Expected workgroup_size(8, 8, 1) in output: {}",
        wgsl
    );
}

#[test]
fn test_workgroup_size_three_args() {
    let source = r#"
@group(0) @binding(0) storage read_write buf: array<f32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    buf[id.x] = 1.0;
}
"#;
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "3-arg workgroup_size failed: {:?}",
        result.err()
    );
}

#[test]
fn test_workgroup_size_with_includes_and_body() {
    let source = r#"
struct Params {
    width: u32,
    height: u32,
    _pad0: f32,
    _pad1: f32,
}

@group(0) @binding(0) uniform params: Params;
@group(0) @binding(1) storage read_write color_buffer: array<u32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.width || id.y >= params.height) {
        return;
    }
    let pixel_idx = id.y * params.width + id.x;
    color_buffer[pixel_idx] = 0u;
}
"#;
    let result = windjammer::wjsl::transpile_wjsl(source);
    assert!(
        result.is_ok(),
        "Workgroup_size with body failed: {:?}",
        result.err()
    );
}
