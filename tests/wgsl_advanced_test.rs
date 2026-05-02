// Advanced WGSL feature tests
// Testing features needed for real production shaders

#[path = "test_utils.rs"]
mod test_utils;

// ============================================================================
// ARRAY INDEXING
// ============================================================================

#[test]
fn test_array_index_read() {
    let source = r#"
pub fn get_value(arr: [uint; 10], idx: uint) -> uint {
    arr[idx]
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(generated.contains("arr[idx]"), "Generated:\n{}", generated);
}

#[test]
fn test_array_index_write() {
    let source = r#"
@compute(workgroup_size = [8, 8, 1])
pub fn update(@builtin(global_invocation_id) id: vec3<uint>) {
    let mut data: [uint; 10] = [0; 10];
    data[id.x] = 42
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("data[id.x] = 42"),
        "Generated:\n{}",
        generated
    );
}

// ============================================================================
// MUTABLE LOCALS (var)
// ============================================================================

#[test]
fn test_mutable_local_var() {
    let source = r#"
pub fn counter() -> uint {
    let mut count = 0;
    count = count + 1;
    count
}
"#;

    let generated = test_utils::compile_single(source);

    // WGSL uses 'var' for mutable locals
    assert!(generated.contains("var count"), "Generated:\n{}", generated);
    assert!(
        generated.contains("count =") && generated.contains("+ 1"),
        "Generated:\n{}",
        generated
    );
}

#[test]
fn test_mutable_with_type_annotation() {
    let source = r#"
pub fn test_var() -> uint {
    let mut x: uint = 10;
    x = x * 2;
    x
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("var x: u32 = 10"),
        "Generated:\n{}",
        generated
    );
}

// ============================================================================
// COMPOUND ASSIGNMENT
// ============================================================================

#[test]
fn test_plus_assign() {
    let source = r#"
pub fn accumulate() {
    let mut sum = 0;
    sum += 10
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("sum += 10") || generated.contains("sum = sum + 10"),
        "Generated:\n{}",
        generated
    );
}

#[test]
fn test_or_assign() {
    let source = r#"
pub fn set_flags() {
    let mut flags = 0;
    flags |= 1;
    flags |= 2
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("flags |= ") || generated.contains("flags = flags |"),
        "Generated:\n{}",
        generated
    );
}

// ============================================================================
// SWIZZLE ACCESS
// ============================================================================

#[test]
fn test_vec3_xyz_swizzle() {
    let source = r#"
pub fn extract(v: vec4<float>) -> vec3<float> {
    v.xyz
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(generated.contains(".xyz"), "Generated:\n{}", generated);
}

#[test]
fn test_vec_component_access() {
    let source = r#"
pub fn get_x(v: vec3<float>) -> float {
    v.x
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(generated.contains(".x"), "Generated:\n{}", generated);
}

// ============================================================================
// BUILTIN FUNCTIONS
// ============================================================================

#[test]
fn test_min_max_builtins() {
    let source = r#"
pub fn clamp_value(x: float, lo: float, hi: float) -> float {
    max(lo, min(x, hi))
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(generated.contains("max("), "Generated:\n{}", generated);
    assert!(generated.contains("min("), "Generated:\n{}", generated);
}

#[test]
fn test_any_builtin() {
    let source = r#"
pub fn out_of_bounds(pos: vec3<float>, bounds: vec3<float>) -> bool {
    any(pos >= bounds)
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(generated.contains("any("), "Generated:\n{}", generated);
}

#[test]
fn test_normalize_builtin() {
    let source = r#"
pub fn unit_vector(v: vec3<float>) -> vec3<float> {
    normalize(v)
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("normalize("),
        "Generated:\n{}",
        generated
    );
}

// ============================================================================
// CAST EXPRESSIONS
// ============================================================================

#[test]
fn test_uint_to_float_cast() {
    let source = r#"
pub fn convert(x: uint) -> float {
    x as float
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(generated.contains("f32("), "Generated:\n{}", generated);
}

#[test]
fn test_float_to_uint_cast() {
    let source = r#"
pub fn convert(x: float) -> uint {
    x as uint
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(generated.contains("u32("), "Generated:\n{}", generated);
}
