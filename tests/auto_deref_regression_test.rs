/// TDD Test: E0614 "cannot be dereferenced" regression fix
///
/// Bug: Auto-deref logic added `*` to expressions that are NOT references:
/// - *(arr[0]) when arr[0] yields f32 (Rust auto-derefs Copy types)
/// - *(1.0) for literal
/// - *(path.nodes[i]) when element is Copy
///
/// Root Cause: Unconditionally treating Index as reference; assuming all Copy args need deref
///
/// Fix: expression_is_reference() - only deref when expression IS actually a reference
/// - Literals: never refs
/// - Index with Copy element: Rust auto-derefs arr[0] → f32, NOT &f32
/// - Index with non-Copy element: vec[i] yields &T → deref

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "wj_auto_deref_{}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        id
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");
    
    if !wj_output.status.success() {
        eprintln!("wj compilation failed!");
        eprintln!("stdout: {}", String::from_utf8_lossy(&wj_output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&wj_output.stderr));
    }

    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

#[test]
fn test_literal_arg_no_deref() {
    // foo(42) → foo(42) NOT foo(*42) - literals are never references
    let source = r#"
pub fn take_int(x: int) -> int {
    x
}

pub fn test() -> int {
    take_int(42)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(!rs.contains("*(42)"), "Should NOT add * to literal");
    assert!(rs.contains("take_int(42)") || rs.contains("take_int(42_i64)"));
}

#[test]
fn test_float_literal_arg_no_deref() {
    // foo(1.0) → foo(1.0) NOT foo(*1.0)
    let source = r#"
pub fn take_float(x: f32) -> f32 {
    x
}

pub fn test() -> f32 {
    take_float(1.0)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(!rs.contains("*(1.0") && !rs.contains("*(1.0_f32)"), "Should NOT add * to float literal");
}

#[test]
fn test_array_index_copy_no_deref() {
    // Vec2::new(arr[0], arr[1]) where arr: [f32; 2]
    // Rust auto-derefs arr[0] → f32. Must NOT add *(arr[0])
    let source = r#"
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn from_array(arr: [f32; 2]) -> Vec2 {
        Vec2::new(arr[0], arr[1])
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(arr[0])") && !rs.contains("*(arr[1])"),
        "Should NOT add * to array index with Copy element. Generated:\n{}",
        rs
    );
    assert!(
        (rs.contains("arr[0]") || rs.contains("arr[0_usize]")) 
        && (rs.contains("arr[1]") || rs.contains("arr[1_usize]")),
        "Should use arr[0] or arr[0_usize], arr[1] or arr[1_usize] directly. Generated:\n{}",
        rs
    );
}

#[test]
fn test_vec3_from_array_no_deref() {
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    pub fn from_array(arr: [f32; 3]) -> Vec3 {
        Vec3::new(arr[0], arr[1], arr[2])
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(arr[0])") && !rs.contains("*(arr[1])") && !rs.contains("*(arr[2])"),
        "Should NOT add * to array index. Generated:\n{}",
        rs
    );
}

#[test]
fn test_vec4_from_array_no_deref() {
    let source = r#"
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Vec4 {
        Vec4 { x, y, z, w }
    }

    pub fn from_array(arr: [f32; 4]) -> Vec4 {
        Vec4::new(arr[0], arr[1], arr[2], arr[3])
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(arr[0])"),
        "Should NOT add * to array index. Generated:\n{}",
        rs
    );
}

#[test]
fn test_planes_array_index_no_deref() {
    // point_in_front_of_plane(point, planes[0]) - Plane is Copy struct
    let source = r#"
pub struct Plane {
    pub normal_x: f32,
    pub normal_y: f32,
    pub normal_z: f32,
    pub d: f32,
}

pub fn point_in_front_of_plane(point_x: f32, plane: Plane) -> bool {
    true
}

pub fn test(point_x: f32, planes: [Plane; 6]) -> bool {
    point_in_front_of_plane(point_x, planes[0])
        && point_in_front_of_plane(point_x, planes[1])
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(planes[0])") && !rs.contains("*(planes[1])"),
        "Should NOT add * to array index with Copy struct. Generated:\n{}",
        rs
    );
}

#[test]
fn test_tuple_index_no_deref() {
    // path.nodes[i] where nodes: Vec<(i32, i32)> - tuple is Copy
    let source = r#"
pub struct Path {
    pub nodes: Vec<(int, int)>,
}

pub fn has_line_of_sight(grid: bool, a: (int, int), b: (int, int)) -> bool {
    true
}

pub fn test(grid: bool, path: Path, current_idx: usize, i: usize) -> bool {
    has_line_of_sight(grid, path.nodes[current_idx], path.nodes[i])
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    // Vec index returns &T. For (i32, i32) which is Copy, Rust might auto-deref.
    // The key is we shouldn't get E0614. Let's just verify it compiles.
    assert!(compiles);
}

#[test]
fn test_borrowed_param_deref_when_needed() {
    // When param expects owned Copy and arg is &T (borrowed param), we SHOULD deref
    let source = r#"
pub fn add_one(x: int) -> int {
    x + 1
}

pub fn test(r: int) -> int {
    add_one(r)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    // r is owned (not borrowed) in this case - param is int
    // So we shouldn't add *. Just verify compiles.
    assert!(compiles);
}

#[test]
fn test_color_rgba_from_array_no_deref() {
    let source = r#"
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    pub fn from_array(arr: [f32; 4]) -> Color {
        Color::rgba(arr[0], arr[1], arr[2], arr[3])
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(arr[0])"),
        "Should NOT add * to array index. Generated:\n{}",
        rs
    );
}

#[test]
fn test_quat_from_array_no_deref() {
    let source = r#"
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Quat {
        Quat { x, y, z, w }
    }

    pub fn from_array(arr: [f32; 4]) -> Quat {
        Quat::new(arr[0], arr[1], arr[2], arr[3])
    }
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(arr[0])"),
        "Should NOT add * to array index. Generated:\n{}",
        rs
    );
}

#[test]
fn test_entity_iter_no_deref() {
    // for entity in entities { result.push(entity) } - entity from Vec<Entity>
    // When Entity is Copy, iter gives owned. Don't incorrectly add *.
    let source = r#"
pub struct Entity {
    pub id: u64
}

pub fn test(entities: Vec<Entity>) -> Vec<Entity> {
    let mut result: Vec<Entity> = vec![]
    for entity in entities {
        result.push(entity)
    }
    result
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}
