// TDD Test: Nested Module Imports
//
// Bug: When generating code for nested modules (core/commands/command.wj),
// imports like `use crate::scene` generate as `use crate::scene` in Rust,
// but they should be `use crate::generated::scene` when output is in src/generated/
//
// Root cause: Codegen doesn't know about the output module path (src/generated/)
// and generates imports relative to crate root, not accounting for the subdirectory.
//
// Expected: Imports should use relative paths within the generated directory:
// - From src/generated/core/commands/ to src/generated/scene.rs
// - Should be: `use super::super::scene` or `use crate::generated::scene`

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_nested_module_imports() {
    let temp_dir = TempDir::new().unwrap();

    // Create scene.wj at root level
    let scene_source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct SceneObject {
    pub id: i32,
    pub position: Vec3,
}
"#;

    // Create nested module that imports from scene
    let command_source = r#"
use crate::scene::{SceneObject, Vec3}

pub struct MoveCommand {
    pub object_id: i32,
    pub old_pos: Vec3,
    pub new_pos: Vec3,
}
"#;

    // Write files
    fs::create_dir_all(temp_dir.path().join("src_wj").join("core").join("commands")).unwrap();
    fs::write(
        temp_dir.path().join("src_wj").join("scene.wj"),
        scene_source,
    )
    .unwrap();
    fs::write(
        temp_dir
            .path()
            .join("src_wj")
            .join("core")
            .join("commands")
            .join("command.wj"),
        command_source,
    )
    .unwrap();

    // Create wj.toml
    let toml_content = r#"
[package]
name = "test_nested_imports"
version = "0.1.0"
"#;
    fs::write(temp_dir.path().join("wj.toml"), toml_content).unwrap();

    // Compile with output to src/generated/
    let output_dir = temp_dir.path().join("src").join("generated");
    fs::create_dir_all(&output_dir).unwrap();

    // Create lib.rs to match real editor structure
    // This makes src/generated/ a submodule, not the crate root
    let lib_rs = "pub mod generated;\n";
    fs::write(temp_dir.path().join("src").join("lib.rs"), lib_rs).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(temp_dir.path().join("src_wj"))
        .arg("-o")
        .arg(&output_dir)
        .arg("--library")
        .arg("--module-file")
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    // Check generated command.rs uses correct imports
    let command_rs = output_dir.join("core").join("commands").join("command.rs");
    let generated = fs::read_to_string(&command_rs).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated command.rs. Build output:\n{}",
            String::from_utf8_lossy(&wj_output.stdout)
        )
    });

    // Should use crate::generated::scene because:
    // - src/lib.rs is the crate root (declares `pub mod generated`)
    // - src/generated/mod.rs is the generated module
    // - From core/commands/command.rs, we need crate::generated::scene
    let has_correct_import = generated.contains("use crate::generated::scene");

    println!("Generated command.rs:\n{}", generated);

    assert!(
        has_correct_import,
        "Expected crate::generated::scene import but got:\n{}",
        generated
    );

    // Verify the generated code actually compiles with Rust
    let cargo_toml = r#"
[package]
name = "test_nested_imports"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    let cargo_output = Command::new("cargo")
        .arg("check")
        .arg("--lib")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run cargo check");

    if !cargo_output.status.success() {
        panic!(
            "Generated code does not compile:\nstdout:\n{}\nstderr:\n{}\n\nGenerated command.rs:\n{}",
            String::from_utf8_lossy(&cargo_output.stdout),
            String::from_utf8_lossy(&cargo_output.stderr),
            generated
        );
    }
}
