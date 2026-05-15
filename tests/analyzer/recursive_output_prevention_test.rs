// TDD: Test that compiler doesn't create recursive directories
// When output is INSIDE source tree, don't copy output directory back into itself

use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_recursive_output_directory() {
    // Setup: output directory is a subdirectory of the project
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create src/components_wj/ (source) and src/components/generated/ (output)
    let source_dir = project_root.join("src/components_wj");
    let output_dir = project_root.join("src/components/generated");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // Create some .wj files
    fs::write(source_dir.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(source_dir.join("input.wj"), "pub struct Input {}").unwrap();

    // Build
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&source_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .current_dir(project_root)
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Check that output directory exists
    assert!(output_dir.exists());

    // Critical: Should NOT create recursive directories
    let recursive_generated = output_dir.join("components/generated");
    if recursive_generated.exists() {
        panic!(
            "Compiler created recursive directory structure!\n\
             Found: {}\n\
             This causes infinite recursion and 'Filename too long' errors on Windows.",
            recursive_generated.display()
        );
    }

    // Should NOT declare components or generated as modules
    let mod_rs_path = output_dir.join("mod.rs");
    if mod_rs_path.exists() {
        let mod_rs_content = fs::read_to_string(&mod_rs_path).unwrap();

        if mod_rs_content.contains("pub mod components;") {
            panic!(
                "mod.rs incorrectly declares 'pub mod components;'\n\
                 This causes 'file not found' errors because it's a recursive directory."
            );
        }

        if mod_rs_content.contains("pub mod generated;") {
            panic!(
                "mod.rs incorrectly declares 'pub mod generated;'\n\
                 This causes 'file not found' errors because it's a recursive directory."
            );
        }
    }
}

// Simplified test removed - main test covers the critical bug

// Simplified test removed - covered by other tests
