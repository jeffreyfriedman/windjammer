// TDD: Comprehensive tests for copying module trees
// Write tests FIRST to explore all edge cases

use std::fs;
use tempfile::TempDir;

#[test]
fn test_module_parent_file_with_submodules_not_copied() {
    // Case 1: events.rs + events/ directory in src/
    // Expected: Neither should be copied to generated/
    // Reason: Generated code imports from crate::events::, not local events

    let temp_dir = TempDir::new().unwrap();
    let project = temp_dir.path();

    let src = project.join("src");
    let src_wj = project.join("src_wj");
    let output = project.join("out");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&src_wj).unwrap();
    fs::create_dir_all(&output).unwrap();

    // Create events.rs with submodule declaration
    fs::write(
        src.join("events.rs"),
        "pub mod dispatcher;\npub use dispatcher::ComponentEventDispatcher;",
    )
    .unwrap();

    // Create events/ subdirectory with dispatcher.rs
    let events_dir = src.join("events");
    fs::create_dir_all(&events_dir).unwrap();
    fs::write(
        events_dir.join("dispatcher.rs"),
        "pub struct ComponentEventDispatcher {}",
    )
    .unwrap();

    // Create .wj source
    fs::write(src_wj.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(src_wj.join("input.wj"), "pub struct Input {}").unwrap();

    // Build
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_wj)
        .arg("-o")
        .arg(&output)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .current_dir(project)
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Check: events.rs should NOT be copied
    let events_rs_copied = output.join("events.rs");
    assert!(
        !events_rs_copied.exists(),
        "events.rs should NOT be copied (has subdirectory)"
    );

    // Check: events/ directory should NOT be copied
    let events_dir_copied = output.join("events");
    assert!(
        !events_dir_copied.exists(),
        "events/ directory should NOT be copied (module parent tree)"
    );

    // Check: mod.rs should NOT declare events
    let mod_rs = output.join("mod.rs");
    if mod_rs.exists() {
        let mod_content = fs::read_to_string(&mod_rs).unwrap();
        assert!(
            !mod_content.contains("pub mod events;"),
            "mod.rs should NOT declare events (module parent with subdirectory)"
        );
    }
}

#[test]
fn test_simple_rs_file_without_subdirectory_is_copied() {
    // Case 2: utils.rs (no utils/ directory)
    // Expected: Should be copied to generated/
    // Reason: Simple hand-written file, no subdirectories

    let temp_dir = TempDir::new().unwrap();
    let project = temp_dir.path();

    let src = project.join("src");
    let src_wj = project.join("src_wj");
    let output = project.join("out");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&src_wj).unwrap();
    fs::create_dir_all(&output).unwrap();

    // Create simple utils.rs (no subdirectory)
    fs::write(src.join("utils.rs"), "pub fn helper() {}").unwrap();

    // Create .wj source
    fs::write(src_wj.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(src_wj.join("input.wj"), "pub struct Input {}").unwrap();

    // Build
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_wj)
        .arg("-o")
        .arg(&output)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .current_dir(project)
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Check: utils.rs SHOULD be copied
    let utils_rs_copied = output.join("utils.rs");
    assert!(
        utils_rs_copied.exists(),
        "utils.rs SHOULD be copied (no subdirectory)"
    );

    // Check: mod.rs SHOULD declare utils
    let mod_rs = output.join("mod.rs");
    if mod_rs.exists() {
        let mod_content = fs::read_to_string(&mod_rs).unwrap();
        assert!(
            mod_content.contains("pub mod utils;"),
            "mod.rs SHOULD declare utils (simple file)"
        );
    }
}

#[test]
fn test_directory_with_mod_rs_not_copied() {
    // Case 3: ffi/ directory with ffi/mod.rs
    // Expected: Should be copied (FFI module)
    // But: If output is inside src/, don't copy directories that contain output

    let temp_dir = TempDir::new().unwrap();
    let project = temp_dir.path();

    let src = project.join("src");
    let src_wj = project.join("src_wj");
    let output = project.join("out");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&src_wj).unwrap();
    fs::create_dir_all(&output).unwrap();

    // Create ffi/ directory with mod.rs
    let ffi_dir = src.join("ffi");
    fs::create_dir_all(&ffi_dir).unwrap();
    fs::write(ffi_dir.join("mod.rs"), "pub fn ffi_call() {}").unwrap();

    // Create .wj source
    fs::write(src_wj.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(src_wj.join("input.wj"), "pub struct Input {}").unwrap();

    // Build
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_wj)
        .arg("-o")
        .arg(&output)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .current_dir(project)
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Check: ffi/ directory SHOULD be copied
    let ffi_dir_copied = output.join("ffi");
    assert!(
        ffi_dir_copied.exists(),
        "ffi/ directory SHOULD be copied (FFI module with mod.rs)"
    );

    // Check: mod.rs SHOULD declare ffi
    let mod_rs = output.join("mod.rs");
    if mod_rs.exists() {
        let mod_content = fs::read_to_string(&mod_rs).unwrap();
        assert!(
            mod_content.contains("pub mod ffi;"),
            "mod.rs SHOULD declare ffi (directory module)"
        );
    }
}

#[test]
fn test_edge_case_both_file_and_directory() {
    // Case 4: What if someone has BOTH config.rs AND config/ directory?
    // This is VALID Rust (config.rs can declare submodules)
    // Expected: Skip BOTH (it's a module parent tree)
    //
    // Reason: Generated code should import from crate::config::, not local

    let temp_dir = TempDir::new().unwrap();
    let project = temp_dir.path();

    let src = project.join("src");
    let src_wj = project.join("src_wj");
    let output = project.join("out");

    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&src_wj).unwrap();
    fs::create_dir_all(&output).unwrap();

    // Create config.rs
    fs::write(
        src.join("config.rs"),
        "pub mod settings;\npub use settings::*;",
    )
    .unwrap();

    // Create config/ directory
    let config_dir = src.join("config");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("settings.rs"), "pub struct Settings {}").unwrap();

    // Create .wj source
    fs::write(src_wj.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(src_wj.join("input.wj"), "pub struct Input {}").unwrap();

    // Build
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_wj)
        .arg("-o")
        .arg(&output)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .current_dir(project)
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Check: config.rs should NOT be copied
    let config_rs_copied = output.join("config.rs");
    assert!(
        !config_rs_copied.exists(),
        "config.rs should NOT be copied (module parent with subdirectory)"
    );

    // Check: config/ directory should NOT be copied
    let config_dir_copied = output.join("config");
    assert!(
        !config_dir_copied.exists(),
        "config/ directory should NOT be copied (module parent tree)"
    );
}

