//! TDD Tests: @derive(Copy) Architecture
//!
//! Verifies that Copy detection comes from @derive(Copy) in source, NOT hardcoded types.
//! The compiler must be application-agnostic - no Entity, Vec3, CameraData, etc.
//!
//! Architecture:
//! - PASS 0: Parses all .wj files, collects @derive(Copy) and all-Copy-field structs
//! - copy_structs_registry: Populated from PASS 0, passed to CodeGenerator
//! - is_known_copy_type: Returns false for ALL types (no hardcoded application types)

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use windjammer::codegen::rust::type_analysis::is_known_copy_type;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "wj_derive_copy_{}_{}_{}",
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

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

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

    (rs_content, rustc.status.success())
}

// =============================================================================
// is_known_copy_type: Must return false for ALL application types
// =============================================================================

#[test]
fn test_is_known_copy_type_returns_false_for_entity() {
    assert!(!is_known_copy_type("Entity"));
    assert!(!is_known_copy_type("EntityId"));
    assert!(!is_known_copy_type("ecs::entity::Entity"));
}

#[test]
fn test_is_known_copy_type_returns_false_for_math_types() {
    assert!(!is_known_copy_type("Vec2"));
    assert!(!is_known_copy_type("Vec3"));
    assert!(!is_known_copy_type("Vec4"));
    assert!(!is_known_copy_type("AABB"));
    assert!(!is_known_copy_type("Rect"));
    assert!(!is_known_copy_type("Point"));
    assert!(!is_known_copy_type("Color"));
}

#[test]
fn test_is_known_copy_type_returns_false_for_app_types() {
    assert!(!is_known_copy_type("CameraData"));
    assert!(!is_known_copy_type("SearchState"));
    assert!(!is_known_copy_type("InvestigationState"));
}

// =============================================================================
// @derive(Copy) in source: copy_structs_registry handles it
// =============================================================================

#[test]
fn test_derive_copy_tracked_in_same_file() {
    let source = r#"
@derive(Copy, Clone)
pub struct MyType { value: i32 }

pub fn process(x: MyType) -> i32 {
    x.value
}

pub fn main() {
    let t = MyType { value: 42 };
    let _ = process(t);
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(x)"),
        "Copy type from @derive(Copy) should NOT get *(x). Generated:\n{}",
        rs
    );
}

#[test]
fn test_no_derive_copy_not_tracked() {
    let source = r#"
pub struct MyType { value: String }

pub fn process(x: &MyType) -> usize {
    x.value.len()
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}

#[test]
fn test_auto_inferred_copy_all_primitive_fields() {
    let source = r#"
pub struct Point3D { x: f32, y: f32, z: f32 }

pub fn distance(p: Point3D) -> f32 {
    (p.x * p.x + p.y * p.y + p.z * p.z).sqrt()
}

pub fn main() {
    let p = Point3D { x: 1.0, y: 2.0, z: 3.0 };
    let _ = distance(p);
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}

#[test]
fn test_for_loop_copy_entity() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct Entity { index: i64 }

pub fn process(entity: Entity) {}

pub fn process_all(entities: Vec<Entity>) {
    for entity in entities {
        process(entity)
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(entity)"),
        "Copy Entity in for-loop should NOT add *(entity). Generated:\n{}",
        rs
    );
}

#[test]
fn test_tuple_pattern_copy_types() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct Entity { index: i64 }

pub struct Mesh {}
pub struct Transform {}

pub fn render_mesh(entity: Entity, mesh: Mesh, transform: Transform) {}

pub fn run_rendering(entities: Vec<(Entity, Mesh, Transform)>) {
    for (entity, mesh, transform) in entities {
        render_mesh(entity, mesh, transform)
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(entity)"),
        "Tuple pattern with Copy Entity should NOT add *(entity). Generated:\n{}",
        rs
    );
}

#[test]
fn test_vec_indexing_copy_struct() {
    let source = r#"
pub struct Vec3 { x: f32, y: f32, z: f32 }

pub fn get_position(positions: Vec<Vec3>, index: i32) -> Vec3 {
    return positions[index]
}

fn main() {
    let positions = Vec::new();
    let _ = get_position(positions, 0);
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("&positions[") && !rs.contains("& positions["),
        "Vec3 (all Copy fields) should NOT add & to indexing. Generated:\n{}",
        rs
    );
}
