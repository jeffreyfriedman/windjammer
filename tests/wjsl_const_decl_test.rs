/// TDD: const declaration support in WJSL transpiler
///
/// Bug: const declarations (e.g., `const TILE_W: u32 = 10u;`) are silently
/// dropped during parsing because the WJSL parser doesn't recognize the
/// `const` keyword. The voxel_denoise.wjsl shader fails because TILE_W,
/// TILE_H, TILE_AREA are undefined when referenced in function bodies.

fn transpile(source: &str) -> Result<String, String> {
    windjammer::wjsl::transpile_wjsl(source).map_err(|e| e.to_string())
}

#[test]
fn test_const_u32_preserved() {
    let source = r#"
const TILE_W: u32 = 10u;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x % TILE_W;
}
"#;
    let result = transpile(source).unwrap();
    assert!(
        result.contains("const TILE_W"),
        "const declaration must be preserved in output: {}",
        result
    );
    assert!(
        result.contains("= 10u"),
        "const initializer must be preserved: {}",
        result
    );
}

#[test]
fn test_const_f32_preserved() {
    let source = r#"
const PI: f32 = 3.14159;

fn circle_area(r: f32) -> f32 {
    return PI * r * r;
}
"#;
    let result = transpile(source).unwrap();
    assert!(
        result.contains("const PI"),
        "const f32 declaration must be preserved: {}",
        result
    );
}

#[test]
fn test_multiple_consts() {
    let source = r#"
const TILE_W: u32 = 10u;
const TILE_H: u32 = 10u;
const TILE_AREA: u32 = 100u;

var<workgroup> tile_data: array<f32, 100>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(local_invocation_id) lid: vec3<u32>) {
    let idx = lid.y * TILE_W + lid.x;
    if (idx < TILE_AREA) {
        tile_data[idx] = 0.0;
    }
}
"#;
    let result = transpile(source).unwrap();
    assert!(
        result.contains("const TILE_W"),
        "TILE_W missing: {}",
        result
    );
    assert!(
        result.contains("const TILE_H"),
        "TILE_H missing: {}",
        result
    );
    assert!(
        result.contains("const TILE_AREA"),
        "TILE_AREA missing: {}",
        result
    );
    assert!(
        result.contains("var<workgroup>"),
        "workgroup var missing: {}",
        result
    );
}

#[test]
fn test_const_with_type_annotation() {
    let source = r#"
const MAX_STEPS: i32 = 128i;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let steps = MAX_STEPS;
}
"#;
    let result = transpile(source).unwrap();
    assert!(
        result.contains("const MAX_STEPS: i32 = 128i;"),
        "const i32 with suffix must be preserved: {}",
        result
    );
}

#[test]
fn test_const_expression_initializer() {
    let source = r#"
const TILE_W: u32 = 8u;
const TILE_H: u32 = 8u;
const TILE_AREA: u32 = 64u;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let area = TILE_AREA;
}
"#;
    let result = transpile(source).unwrap();
    assert!(
        result.contains("const TILE_W: u32 = 8u;"),
        "output: {}",
        result
    );
    assert!(
        result.contains("const TILE_H: u32 = 8u;"),
        "output: {}",
        result
    );
    assert!(
        result.contains("const TILE_AREA: u32 = 64u;"),
        "output: {}",
        result
    );
}
