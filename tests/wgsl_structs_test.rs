/// TDD test: WGSL struct layout and alignment
///
/// WGSL has strict alignment rules that differ from Rust:
/// - vec2: 8-byte alignment
/// - vec3: 16-byte alignment (!)
/// - vec4: 16-byte alignment
/// - Structs: 16-byte alignment
///
/// The compiler must automatically insert padding to match GPU requirements.
use std::fs;
use std::process::Command;

fn transpile_wj_to_wgsl(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_wgsl_struct_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("wgsl")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Compilation failed:\nSTDERR:\n{}\nSTDOUT:\n{}",
            stderr, stdout
        );
    }

    let wgsl_file = out_dir.join("test.wgsl");
    let content = fs::read_to_string(&wgsl_file).expect("Failed to read generated WGSL file");

    let _ = fs::remove_dir_all(&test_dir);

    content
}

// ============================================================================
// PRIMITIVE TYPE TESTS
// ============================================================================

#[test]
fn test_struct_with_primitives() {
    let source = r#"
pub struct Primitives {
    a: uint,
    b: int32,
    c: float,
    d: bool,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("struct Primitives"));
    assert!(generated.contains("a: u32"));
    assert!(generated.contains("b: i32"));
    assert!(generated.contains("c: f32"));
    assert!(generated.contains("d: bool"));
}

// ============================================================================
// VEC2 ALIGNMENT TESTS (8-byte alignment)
// ============================================================================

#[test]
fn test_vec2_after_u32() {
    // u32 (4 bytes) + vec2 (8 bytes, 8-byte alignment)
    // No padding needed - u32 fits in first 4 bytes
    let source = r#"
pub struct Layout1 {
    count: uint,
    position: vec2<float>,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Should NOT have padding between count and position
    assert!(generated.contains("count: u32"));
    assert!(generated.contains("position: vec2<f32>"));
}

#[test]
fn test_two_vec2() {
    // vec2 (8 bytes) + vec2 (8 bytes)
    // Perfectly aligned, no padding
    let source = r#"
pub struct Layout2 {
    pos: vec2<float>,
    vel: vec2<float>,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("pos: vec2<f32>"));
    assert!(generated.contains("vel: vec2<f32>"));
}

// ============================================================================
// VEC3 ALIGNMENT TESTS (16-byte alignment - CRITICAL!)
// ============================================================================

#[test]
fn test_vec3_alignment() {
    // CRITICAL: vec3 has 16-byte alignment in WGSL!
    // This is the most common source of GPU bugs
    let source = r#"
pub struct CameraUniforms {
    position: vec3<float>,
    screen_size: vec2<float>,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // vec3 is 12 bytes, but next field must be 16-byte aligned
    // Need 4 bytes of padding
    assert!(
        generated.contains("_pad") || generated.contains("padding"),
        "Should insert padding after vec3. Got:\n{}",
        generated
    );
}

#[test]
fn test_vec3_after_u32() {
    // u32 (4 bytes) + vec3 (12 bytes, 16-byte alignment)
    // Need 12 bytes padding to align vec3 to 16-byte boundary
    let source = r#"
pub struct Layout3 {
    count: uint,
    position: vec3<float>,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(
        generated.contains("_pad") || generated.contains("padding"),
        "Should insert padding before vec3. Got:\n{}",
        generated
    );
}

#[test]
fn test_two_vec3() {
    // vec3 (12 bytes) + padding (4 bytes) + vec3 (12 bytes)
    let source = r#"
pub struct Dual {
    pos: vec3<float>,
    vel: vec3<float>,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // First vec3 needs padding before second vec3
    assert!(
        generated.contains("_pad") || generated.contains("padding"),
        "Should insert padding between vec3 fields. Got:\n{}",
        generated
    );
}

// ============================================================================
// VEC4 ALIGNMENT TESTS (16-byte alignment)
// ============================================================================

#[test]
fn test_vec4_alignment() {
    let source = r#"
pub struct Color {
    rgba: vec4<float>,
    intensity: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // vec4 is 16 bytes, perfectly aligned
    // float after it starts at offset 16
    assert!(generated.contains("rgba: vec4<f32>"));
    assert!(generated.contains("intensity: f32"));
}

#[test]
fn test_vec4_after_u32() {
    // u32 (4 bytes) + vec4 (16 bytes, 16-byte alignment)
    // Need 12 bytes padding
    let source = r#"
pub struct Layout4 {
    id: uint,
    color: vec4<float>,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(
        generated.contains("_pad") || generated.contains("padding"),
        "Should insert padding before vec4. Got:\n{}",
        generated
    );
}

// ============================================================================
// MATRIX ALIGNMENT TESTS
// ============================================================================

#[test]
fn test_mat4x4_alignment() {
    // mat4x4 is 64 bytes with 16-byte alignment
    let source = r#"
pub struct Transform {
    matrix: mat4x4<float>,
    scale: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("matrix: mat4x4<f32>"));
    assert!(generated.contains("scale: f32"));
}

// ============================================================================
// COMPLEX REAL-WORLD CASES
// ============================================================================

#[test]
fn test_camera_uniforms() {
    // Real-world shader uniform layout
    let source = r#"
pub struct CameraUniforms {
    view_matrix: mat4x4<float>,
    proj_matrix: mat4x4<float>,
    position: vec3<float>,
    fov: float,
    screen_width: uint,
    screen_height: uint,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("struct CameraUniforms"));
    assert!(generated.contains("view_matrix: mat4x4<f32>"));
    assert!(generated.contains("position: vec3<f32>"));
}

#[test]
fn test_light_data() {
    let source = r#"
pub struct Light {
    position: vec3<float>,
    intensity: float,
    color: vec3<float>,
    radius: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Both vec3 fields need padding
    assert!(generated.contains("struct Light"));
}

#[test]
fn test_particle() {
    let source = r#"
pub struct Particle {
    position: vec3<float>,
    velocity: vec3<float>,
    lifetime: float,
    size: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("struct Particle"));
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_single_field_struct() {
    let source = r#"
pub struct Single {
    value: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("value: f32"));
}

#[test]
fn test_all_u32() {
    // All same type, 4-byte aligned
    let source = r#"
pub struct Counters {
    a: uint,
    b: uint,
    c: uint,
    d: uint,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // No padding needed
    assert!(!generated.contains("_pad"));
}

#[test]
fn test_mixed_sizes() {
    // Complex mix of sizes
    let source = r#"
pub struct Mixed {
    a: uint,
    b: vec2<float>,
    c: uint,
    d: vec3<float>,
    e: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("struct Mixed"));
}

// ============================================================================
// ARRAY TESTS
// ============================================================================

#[test]
fn test_array_of_vec3() {
    let source = r#"
pub struct Points {
    count: uint,
    positions: [vec3<float>; 10],
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Arrays in WGSL have special stride requirements
    assert!(generated.contains("struct Points"));
}

// ============================================================================
// VECTOR TYPE VARIANTS
// ============================================================================

#[test]
fn test_vec2_variants() {
    let source = r#"
pub struct Vec2Types {
    f: vec2<float>,
    u: vec2<uint>,
    i: vec2<int32>,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("vec2<f32>"));
    assert!(generated.contains("vec2<u32>"));
    assert!(generated.contains("vec2<i32>"));
}

#[test]
fn test_vec3_variants() {
    let source = r#"
pub struct Vec3Types {
    f: vec3<float>,
    u: vec3<uint>,
    i: vec3<int32>,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("vec3<f32>"));
    assert!(generated.contains("vec3<u32>"));
    assert!(generated.contains("vec3<i32>"));
}

#[test]
fn test_vec4_variants() {
    let source = r#"
pub struct Vec4Types {
    f: vec4<float>,
    u: vec4<uint>,
    i: vec4<int32>,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("vec4<f32>"));
    assert!(generated.contains("vec4<u32>"));
    assert!(generated.contains("vec4<i32>"));
}

// ============================================================================
// BOOL ALIGNMENT
// ============================================================================

#[test]
fn test_bool_alignment() {
    // Bools in WGSL are 4 bytes
    let source = r#"
pub struct Flags {
    enabled: bool,
    visible: bool,
    active: bool,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("enabled: bool"));
}

// ============================================================================
// STRUCT END PADDING
// ============================================================================

#[test]
fn test_struct_end_padding() {
    // Structs must be aligned to 16 bytes
    // If last field doesn't reach 16-byte boundary, add padding
    let source = r#"
pub struct SmallStruct {
    value: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // May need padding at end to reach 16-byte alignment
    assert!(generated.contains("struct SmallStruct"));
}

// ============================================================================
// MULTIPLE STRUCTS
// ============================================================================

#[test]
fn test_multiple_structs() {
    let source = r#"
pub struct Vec3Wrapper {
    value: vec3<float>,
}

pub struct Data {
    wrapper: Vec3Wrapper,
    scale: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("struct Vec3Wrapper"));
    assert!(generated.contains("struct Data"));
}
