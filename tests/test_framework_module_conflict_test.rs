#[path = "test_utils.rs"]
mod test_utils;

use anyhow::Result;
/// TDD Test: Test framework should not create module conflicts
///
/// PROBLEM: When `build_project` generates a library with a file named `lib.wj`,
/// the test framework creates:
/// - lib/lib.rs (generated from lib.wj)
/// - lib/lib.rs (library entry point that declares `pub mod lib;`)
///
/// This causes E0761: file for module `lib` found at both "lib.rs" and "lib/mod.rs"
///
/// FIX: The `generate_lib_rs_for_library` function should skip any module
/// named "lib" when generating the library entry point, as "lib" is a reserved
/// name for the library itself.
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_lib_module_no_conflict() -> Result<()> {
    let temp_dir = tempdir().expect("tempdir").path().to_path_buf();
    fs::create_dir_all(&temp_dir)?;

    // Create src directory with a lib.wj file
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Create lib.wj with a simple struct
    let lib_wj = src_dir.join("lib.wj");
    fs::write(
        &lib_wj,
        r#"
struct LibConfig {
    name: String,
    version: i32,
}

impl LibConfig {
    fn new(name: String) -> LibConfig {
        LibConfig {
            name: name,
            version: 1,
        }
    }
}
"#,
    )?;

    // Create another module to ensure lib.rs is generated
    let math_wj = src_dir.join("math.wj");
    fs::write(
        &math_wj,
        r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )?;

    // Compile the library directly (skip test framework for now)
    let wj_compiler = test_utils::wj_binary();
    let lib_output = temp_dir.join("lib");
    fs::create_dir_all(&lib_output)?;

    let output = Command::new(&wj_compiler)
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&lib_output)
        .arg("--library")
        .arg("--no-cargo")
        .output()?;

    let _stderr = String::from_utf8_lossy(&output.stderr);
    let _stdout = String::from_utf8_lossy(&output.stdout);

    // Check the generated lib.rs file
    let lib_rs_path = lib_output.join("lib.rs");
    if lib_rs_path.exists() {
        let lib_rs_content = fs::read_to_string(&lib_rs_path)?;

        // The lib.rs should NOT declare "pub mod lib;"
        // because "lib" is the library itself, not a module to import
        assert!(
            !lib_rs_content.contains("pub mod lib;"),
            "Generated lib.rs should NOT contain 'pub mod lib;' - this creates E0761 conflict.\nlib.rs content:\n{}",
            lib_rs_content
        );

        println!("✓ lib.rs does not contain 'pub mod lib;' - correct!");
    } else {
        // If lib.rs doesn't exist, that's also an error
        panic!("lib.rs was not generated at {:?}", lib_rs_path);
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

/// `window.wj` + `window/manager.wj` must not produce `window.rs` and `window/mod.rs` for the same module.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_window_module_no_conflict() -> Result<()> {
    let temp_dir = tempdir().expect("tempdir").path().to_path_buf();
    fs::create_dir_all(&temp_dir)?;

    // Create src directory with both window.wj and window/ directory
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Create window.wj
    let window_wj = src_dir.join("window.wj");
    fs::write(
        &window_wj,
        r#"
struct Window {
    title: String,
    width: i32,
    height: i32,
}
"#,
    )?;

    // Create window/ directory with nested module
    let window_dir = src_dir.join("window");
    fs::create_dir_all(&window_dir)?;

    let window_manager_wj = window_dir.join("manager.wj");
    fs::write(
        &window_manager_wj,
        r#"
struct WindowManager {
    windows: Vec<i32>,
}
"#,
    )?;

    // Create tests_wj directory with a test file
    let tests_wj_dir = temp_dir.join("tests_wj");
    fs::create_dir_all(&tests_wj_dir)?;

    let test_wj = tests_wj_dir.join("window_test.wj");
    fs::write(
        &test_wj,
        r#"
@test
fn test_window_module() {
    // This test should compile without E0761 errors
    assert!(true);
}
"#,
    )?;

    // Create wj.toml
    let wj_toml = temp_dir.join("wj.toml");
    fs::write(
        &wj_toml,
        r#"
[package]
name = "test_window_conflict"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Run wj test
    let wj_compiler = test_utils::wj_binary();
    let output = Command::new(&wj_compiler)
        .arg("test")
        .arg(&test_wj)
        .current_dir(&temp_dir)
        .output()?;

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);

    // Check for E0761 error
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT have E0761 error for window
    assert!(
        !stderr.contains("file for module `window` found at both") && !stdout.contains("file for module `window` found at both"),
        "Test framework should not create window module conflicts. Found conflict error:\nSTDOUT:\n{}\nSTDERR:\n{}",
        stdout,
        stderr
    );

    Ok(())
}
