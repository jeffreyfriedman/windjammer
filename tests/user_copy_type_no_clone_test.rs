/// TDD Test: No .clone() on user-defined Copy types (enums/structs with @derive(Copy))
///
/// Bug: When a variable of a user-defined Copy type (e.g., VoxelType with @derive(Copy))
/// is used multiple times, the auto-clone analysis inserts `.clone()` before the first use
/// to keep the value available for the second use. But Copy types are implicitly copied
/// in Rust, so `.clone()` is unnecessary noise.
///
/// Root Cause: `is_copy_type` only recognized primitive Copy types (i32, f32, bool, etc.)
/// and didn't know about user-defined types with @derive(Copy).
///
/// Fix: Added `copy_types_registry` to CodeGenerator, populated from the global
/// `copy_structs_registry` in ModuleCompiler. The new `is_type_copy()` helper checks
/// both primitive types AND the registry.

use std::io::Write;
use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = dir.path().join("test.wj");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut file = std::fs::File::create(&wj_path).unwrap();
    file.write_all(source.as_bytes()).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "--no-cargo",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let rs_path = out_dir.join("test.rs");
    if rs_path.exists() {
        std::fs::read_to_string(&rs_path).unwrap()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "No output file generated.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
}

#[test]
fn test_no_clone_on_derive_copy_enum_multi_use() {
    // Pattern from windjammer-game rendering/mesh_generator.wj:
    // let voxel = chunk.get_local_voxel(x, y, z)
    // if voxel.is_solid() && condition {
    //     mask[y][z] = Some(voxel)  // voxel used again
    // }
    // VoxelType has @derive(Copy) — .clone() is unnecessary
    let source = r#"
@derive(Copy, Clone, PartialEq)
pub enum VoxelType {
    Air,
    Stone,
    Water,
}

impl VoxelType {
    pub fn is_solid(self) -> bool {
        match self {
            VoxelType::Air => false,
            VoxelType::Water => false,
            _ => true,
        }
    }
}

pub fn process_voxel(voxel_type: VoxelType) -> bool {
    let result = voxel_type.is_solid()
    result
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // VoxelType derives Copy — should NOT need .clone()
    assert!(
        !generated.contains("voxel_type.clone()"),
        "Should not clone @derive(Copy) enum variable.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_derive_copy_struct_multi_use() {
    // A struct with all Copy fields and @derive(Copy) should not need .clone()
    let source = r#"
@derive(Copy, Clone, PartialEq)
pub struct Point {
    pub x: f32,
    pub y: f32,
}

pub fn distance(a: Point, b: Point) -> f32 {
    let dx = a.x - b.x
    let dy = a.y - b.y
    dx * dx + dy * dy
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // Point derives Copy — a.x and b.x should not trigger .clone()
    assert!(
        !generated.contains("a.clone()"),
        "Should not clone @derive(Copy) struct parameter.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("b.clone()"),
        "Should not clone @derive(Copy) struct parameter.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_clone_still_needed_for_non_copy_type() {
    // Ensure non-Copy types still get .clone() when needed
    let source = r#"
pub struct Item {
    pub name: string,
    pub value: i32,
}

pub fn use_item_twice(item: Item) -> string {
    let name = item.name.clone()
    name
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    // Item does NOT derive Copy (has String field) — .clone() IS expected
    assert!(
        generated.contains(".clone()"),
        "Non-Copy type should still use .clone() when needed.\nGenerated:\n{}",
        generated
    );
}
