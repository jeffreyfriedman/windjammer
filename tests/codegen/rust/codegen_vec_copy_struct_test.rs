#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// TDD Test: Vec indexing with Copy structs (Vec3, AABB) should NOT add &
///
/// Bug: Vec indexing auto-borrowed Copy types (Vec3, AABB) causing E0308
///      "expected Vec3, found &Vec3" errors in breach-protocol
///
/// Root Cause: is_copy_type only checked primitives, not user structs with
///             @derive(Copy) or structs with all-Copy fields
///
/// Fix: Enhanced is_type_copy to check:
/// - copy_types_registry (@derive(Copy))
/// - struct_field_types (recursive: all fields Copy)
/// - is_known_copy_type (Vec3, AABB from external crates)
#[path = "../../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec3_is_copy_direct_indexing() {
    // Vec3 (all f32 fields) is Copy - should generate positions[i] NOT &positions[i]
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn get_position(positions: Vec<Vec3>, index: i32) -> Vec3 {
    return positions[index]
}

fn main() {
    let positions = Vec::new()
    let _ = get_position(positions, 0)
}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    // Copy structs should NOT be borrowed - direct indexing
    assert!(
        !rust.contains("&positions[") && !rust.contains("& positions["),
        "Vec3 is Copy, should NOT add & to Vec indexing. Got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_aabb_is_copy_direct_indexing() {
    // AABB (all f32 fields) is Copy - should generate walls[i] NOT &walls[i]
    let source = r#"
pub struct AABB {
    pub min_x: f32,
    pub min_y: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub max_z: f32,
}

pub fn get_wall(walls: Vec<AABB>, i: i32) -> AABB {
    return walls[i]
}

fn main() {}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    assert!(
        !rust.contains("&walls[") && !rust.contains("& walls["),
        "AABB is Copy, should NOT add & to Vec indexing. Got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_string_is_not_copy_gets_borrow() {
    // String is NOT Copy - should generate &names[i] for borrow context
    let source = r#"
pub fn get_name(names: Vec<string>, i: i32) -> string {
    let n = names[i]
    return n
}

fn main() {}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    // Non-Copy should get & or .clone()
    assert!(
        rust.contains("&names[") || rust.contains("& names[") || rust.contains(".clone()"),
        "String not Copy - should borrow or clone. Got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}
