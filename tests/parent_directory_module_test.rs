// TDD: Test that compiler doesn't declare parent directories as modules
// When output is src/components/generated/, don't declare "pub mod components;"

use std::fs;
use tempfile::TempDir;

#[test]
fn test_no_parent_directory_module_declaration() {
    let temp_dir = TempDir::new().unwrap();
    let project = temp_dir.path();

    // Create structure: src/components/ (with hand-written .rs) and src/components_wj/
    let components_dir = project.join("src/components");
    let components_wj = project.join("src/components_wj");
    let output_dir = project.join("src/components/generated");

    fs::create_dir_all(&components_dir).unwrap();
    fs::create_dir_all(&components_wj).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // Create hand-written .rs file in src/components/
    fs::write(components_dir.join("button.rs"), "pub struct Button {}").unwrap();

    // Create .wj sources (need at least 2 for mod.rs generation)
    fs::write(components_wj.join("input.wj"), "pub struct Input {}").unwrap();
    fs::write(components_wj.join("form.wj"), "pub struct Form {}").unwrap();

    // Build
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&components_wj)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .current_dir(project)
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Check mod.rs
    let mod_rs = output_dir.join("mod.rs");
    assert!(mod_rs.exists(), "mod.rs should be generated");

    let mod_content = fs::read_to_string(&mod_rs).unwrap();

    // Should NOT declare components as a module (it's the parent directory!)
    if mod_content.contains("pub mod components;") {
        panic!(
            "mod.rs incorrectly declares 'pub mod components;'\n\
             This is the PARENT directory of output (src/components/generated/)\n\
             and cannot be referenced as a submodule.\n\
             \n\
             mod.rs content:\n{}",
            mod_content
        );
    }

    // SHOULD declare input (the generated module)
    assert!(
        mod_content.contains("pub mod input;"),
        "mod.rs should declare input module"
    );
}
