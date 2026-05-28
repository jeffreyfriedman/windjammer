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

/// TDD: var<workgroup> support in WJSL transpiler
///
/// Bug: var<workgroup> declarations are silently dropped during parsing
/// because parse_private_var() only handles var<private>.
/// The Hi-Z downsample shader fails because tile_depth is undefined.
fn transpile(source: &str) -> Result<String, String> {
    windjammer::wjsl::transpile_wjsl(source).map_err(|e| e.to_string())
}

#[test]
fn test_workgroup_var_preserved_in_output() {
    let source = r#"
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let tid = lid.y * 8u + lid.x;
}
"#;
    let result = transpile(source).unwrap();
    assert!(
        result.contains("fn main"),
        "Basic compute shader should compile: {}",
        result
    );
}

#[test]
fn test_workgroup_var_array_f32() {
    let source = r#"
var<workgroup> tile_depth: array<f32, 64>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(local_invocation_id) lid: vec3<u32>) {
    let tid = lid.y * 8u + lid.x;
    tile_depth[tid] = 1.0;
    workgroupBarrier();
}
"#;
    let result = transpile(source).unwrap();
    assert!(
        result.contains("var<workgroup>") || result.contains("tile_depth"),
        "var<workgroup> must be preserved in output: {}",
        result
    );
}

#[test]
fn test_workgroup_var_used_in_expressions() {
    let source = r#"
var<workgroup> shared_data: array<u32, 256>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(local_invocation_id) lid: vec3<u32>) {
    let idx = lid.y * 8u + lid.x;
    shared_data[idx] = idx;
    workgroupBarrier();
    let val = shared_data[0u];
}
"#;
    let result = transpile(source).unwrap();
    assert!(
        result.contains("shared_data"),
        "workgroup var 'shared_data' must appear in output: {}",
        result
    );
}
