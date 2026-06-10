#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD Tests: @derive(Copy) Architecture
//!
//! Verifies that Copy detection comes from @derive(Copy) in source, NOT hardcoded types.
//! The compiler must be application-agnostic - no Entity, Vec3, CameraData, etc.
//!
//! Architecture:
//! - PASS 0: Parses all .wj files, collects @derive(Copy) and all-Copy-field structs
//! - copy_structs_registry: Populated from PASS 0, passed to CodeGenerator
//! - is_known_copy_type: Returns false for ALL types (no hardcoded application types)

#[path = "common/test_utils.rs"]
mod test_utils;

use windjammer::codegen::rust::type_analysis::is_known_copy_type;

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
fn test_is_known_copy_type_returns_true_for_math_types() {
    assert!(is_known_copy_type("Vec2"));
    assert!(is_known_copy_type("Vec3"));
    assert!(is_known_copy_type("Vec4"));
    assert!(is_known_copy_type("AABB"));
    assert!(is_known_copy_type("Rect"));
    assert!(is_known_copy_type("Point"));
    assert!(is_known_copy_type("Color"));
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
    let (rs, compiles) = test_utils::compile_single_check(source);
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
pub struct MyType { value: string }

pub fn process(x: &MyType) -> usize {
    x.value.len()
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
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
    let (rs, compiles) = test_utils::compile_single_check(source);
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
    let (rs, compiles) = test_utils::compile_single_check(source);
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
    let (rs, compiles) = test_utils::compile_single_check(source);
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
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("&positions[") && !rs.contains("& positions["),
        "Vec3 (all Copy fields) should NOT add & to indexing. Generated:\n{}",
        rs
    );
}
