// TDD: Test that compiler generates mod.rs (not lib.rs) when output is a subdirectory
// Write tests FIRST, then implement the fix

use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generates_mod_rs_for_subdirectory_output() {
    // When generating into a subdirectory (like src/components/generated),
    // should generate mod.rs, not lib.rs
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a project structure with subdirectories
    let components_wj = project_root.join("src/components_wj");
    let components_gen = project_root.join("src/components/generated");
    fs::create_dir_all(&components_wj).unwrap();
    fs::create_dir_all(&components_gen).unwrap();

    // Create some .wj files
    fs::write(components_wj.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(components_wj.join("input.wj"), "pub struct Input {}").unwrap();

    // Build with output to subdirectory
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&components_wj)
        .arg("-o")
        .arg(&components_gen)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .current_dir(project_root)
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Should generate mod.rs (for submodule), not lib.rs (for crate root)
    let mod_rs_path = components_gen.join("mod.rs");
    let lib_rs_path = components_gen.join("lib.rs");

    if lib_rs_path.exists() {
        // Read what was generated to help debug
        let lib_content = fs::read_to_string(&lib_rs_path).unwrap();
        let mod_exists = mod_rs_path.exists();

        panic!(
            "Should generate mod.rs for subdirectory output, not lib.rs!\n\
             lib.rs exists: true\n\
             mod.rs exists: {}\n\
             lib.rs content:\n{}",
            mod_exists, lib_content
        );
    }

    assert!(
        mod_rs_path.exists(),
        "mod.rs should be generated for subdirectory output"
    );

    // Verify mod.rs has correct content
    let mod_content = fs::read_to_string(&mod_rs_path).unwrap();
    assert!(mod_content.contains("pub mod button;"));
    assert!(mod_content.contains("pub mod input;"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generates_lib_rs_for_crate_root() {
    // When generating into a crate root (top-level output),
    // should generate lib.rs
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("src_wj");
    let output_dir = temp_dir.path().join("out");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // Create multiple .wj files (need at least 2 for lib.rs generation)
    fs::write(
        source_dir.join("math.wj"),
        "pub fn add(a: int, b: int) -> int { a + b }",
    )
    .unwrap();
    fs::write(source_dir.join("utils.wj"), "pub fn helper() {}").unwrap();

    // Build to output directory
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&source_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Should generate lib.rs for crate root
    let lib_rs_path = output_dir.join("lib.rs");
    assert!(
        lib_rs_path.exists(),
        "lib.rs should be generated for crate root output"
    );

    let lib_content = fs::read_to_string(&lib_rs_path).unwrap();
    assert!(lib_content.contains("pub mod math;"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_detects_subdirectory_by_parent_src() {
    // When output path contains "src/", it's likely a subdirectory
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create src/generated structure
    let source = project_root.join("components_wj");
    let output = project_root.join("src/generated");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&output).unwrap();

    // Create multiple .wj files (need at least 2 for mod.rs generation)
    fs::write(source.join("widget.wj"), "pub struct Widget {}").unwrap();
    fs::write(source.join("button.wj"), "pub struct Button {}").unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Should generate mod.rs because output is in src/
    let mod_rs_path = output.join("mod.rs");
    assert!(
        mod_rs_path.exists(),
        "mod.rs should be generated when output is in src/ directory"
    );
}
