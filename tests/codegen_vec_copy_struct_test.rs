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

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust_and_check(source: &str, _test_name: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "wj_vec_copy_struct_{}_{}_{}",
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

    let bin_output = dir.join("test_bin");
    let rustc = Command::new("rustc")
        .args(["--edition", "2021", "-o", bin_output.to_str().unwrap()])
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

    let (rust, compiles) = compile_wj_to_rust_and_check(source, "vec3");

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

    let (rust, compiles) = compile_wj_to_rust_and_check(source, "aabb");

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

    let (rust, compiles) = compile_wj_to_rust_and_check(source, "string");

    // Non-Copy should get & or .clone()
    assert!(
        rust.contains("&names[") || rust.contains("& names[") || rust.contains(".clone()"),
        "String not Copy - should borrow or clone. Got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}
