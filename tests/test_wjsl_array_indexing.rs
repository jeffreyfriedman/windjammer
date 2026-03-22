//! TDD tests for WJSL array indexing in function bodies
//!
//! Bug: "Expected semicolon, found LBracket" when parsing arr[idx] syntax

use windjammer::wjsl::transpile_wjsl;

#[test]
fn test_array_indexing_in_function_body() {
    let source = r#"
@group(0) @binding(0) storage read clusters: array<vec4>;
@group(0) @binding(1) storage read_write instances: array<u32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cluster_id = id.x;
    let cluster = clusters[cluster_id];
    instances[cluster_id] = 1u;
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("clusters[cluster_id]"), "WGSL should preserve array indexing");
    assert!(wgsl.contains("instances[cluster_id]"));
}

#[test]
fn test_array_indexing_with_field_access() {
    let source = r#"
struct Item { value: u32 }
@group(0) @binding(0) storage read items: array<Item>;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    let val = items[idx].value;
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("items[idx]"));
}

#[test]
fn test_matrix_indexing() {
    let source = r#"
@group(0) @binding(0) uniform camera: mat4x4;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let row = camera[0];
    let elem = camera[0][3];
}
"#;
    let wgsl = transpile_wjsl(source).unwrap();
    assert!(wgsl.contains("camera[0]"));
}
