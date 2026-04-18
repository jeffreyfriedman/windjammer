// TDD Test: Cross-file field access on Copy-type fields should NOT add .clone()
//
// Root cause: The library build collects global_struct_fields (struct → field types)
// but never passes it to the CodeGenerator. This means the codegen's infer_expression_type
// can't determine that `pass.pass_id` yields a Copy-type `PassId`, so auto_clone_analysis
// adds an unnecessary .clone().
//
// Expected behavior:
//   `pass.pass_id` where PassId is Copy → NO .clone() added
//
// This test MUST FAIL before the fix and PASS after.

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_cross_file_copy_enum_field_no_clone() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    // File 1: Define a Copy enum and a struct containing it
    std::fs::write(
        src.join("types.wj"),
        r#"
@derive(Copy, Clone, Debug, PartialEq)
pub enum PassId {
    Raycast,
    Lighting,
    Composite
}

pub struct CompiledPass {
    pub pass_id: PassId,
    pub label: string
}
"#,
    )
    .unwrap();

    // File 2: Access the Copy-type field from another file
    std::fs::write(
        src.join("executor.wj"),
        r#"
use crate::types::PassId
use crate::types::CompiledPass

pub fn pass_id_to_label(pass_id: PassId) -> string {
    match pass_id {
        PassId::Raycast => "raycast",
        PassId::Lighting => "lighting",
        PassId::Composite => "composite"
    }
}

pub fn execute(pass: CompiledPass) {
    let label = pass_id_to_label(pass.pass_id)
    println!("{}", label)
}
"#,
    )
    .unwrap();

    let result = build_project_ext(
        &src,
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    );
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let executor_rs = std::fs::read_to_string(build.join("executor.rs")).unwrap();

    eprintln!("=== GENERATED executor.rs ===\n{}", executor_rs);

    // The key assertion: pass.pass_id should NOT have .clone()
    // PassId is Copy, so the compiler should pass it by value
    assert!(
        !executor_rs.contains("pass.pass_id.clone()"),
        "Cross-file Copy-type field access should NOT add .clone().\nGenerated:\n{}",
        executor_rs
    );

    // Should contain pass_id_to_label(pass.pass_id) without clone
    assert!(
        executor_rs.contains("pass_id_to_label(pass.pass_id)"),
        "Should call pass_id_to_label with pass.pass_id (no clone).\nGenerated:\n{}",
        executor_rs
    );
}

#[test]
fn test_cross_file_copy_struct_field_no_clone() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    // File 1: Define Copy struct and a container struct
    std::fs::write(
        src.join("math.wj"),
        r#"
@derive(Copy, Clone, Debug)
pub struct Vec2 {
    pub x: f32,
    pub y: f32
}

pub struct Transform {
    pub position: Vec2,
    pub name: string
}
"#,
    )
    .unwrap();

    // File 2: Access Copy-type field
    std::fs::write(
        src.join("physics.wj"),
        r#"
use crate::math::Vec2
use crate::math::Transform

pub fn get_x(pos: Vec2) -> f32 {
    pos.x
}

pub fn process(t: Transform) -> f32 {
    let x = get_x(t.position)
    x
}
"#,
    )
    .unwrap();

    let result = build_project_ext(
        &src,
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    );
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let physics_rs = std::fs::read_to_string(build.join("physics.rs")).unwrap();

    eprintln!("=== GENERATED physics.rs ===\n{}", physics_rs);

    assert!(
        !physics_rs.contains("t.position.clone()"),
        "Cross-file Copy-type struct field should NOT add .clone().\nGenerated:\n{}",
        physics_rs
    );
}
