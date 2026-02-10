// TDD: Test that compiler detects and prevents ambiguous glob re-exports
// Write tests FIRST, then implement detection/warning logic

use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_detect_duplicate_struct_exports() {
    // Two modules exporting structs with the same name
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Both modules export "Config"
    fs::write(
        src_dir.join("module_a.wj"),
        r#"
pub struct Config {
    value: int,
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("module_b.wj"),
        r#"
pub struct Config {
    setting: string,
}
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--module-file")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    let mod_rs = fs::read_to_string(output_dir.join("mod.rs")).unwrap();
    println!("Generated mod.rs:\n{}", mod_rs);

    // Check if generated mod.rs would cause ambiguous re-exports
    let has_ambiguous =
        mod_rs.contains("pub use module_a::*;") && mod_rs.contains("pub use module_b::*;");

    if has_ambiguous {
        // Try to compile the generated code
        let compile_result = std::process::Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg(output_dir.join("mod.rs"))
            .arg("--edition")
            .arg("2021")
            .arg("-L")
            .arg(&output_dir)
            .current_dir(&output_dir)
            .output();

        if let Ok(output) = compile_result {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("Rustc output:\n{}", stderr);

            // Should get a warning/error about ambiguous re-exports
            assert!(
                stderr.contains("ambiguous") || stderr.contains("multiple applicable items"),
                "Expected ambiguous re-export error, but rustc succeeded"
            );
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unique_exports_no_ambiguity() {
    // Modules with unique exports should use glob re-exports
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    fs::write(src_dir.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(src_dir.join("input.wj"), "pub struct Input {}").unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--module-file")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    let mod_rs = fs::read_to_string(output_dir.join("mod.rs")).unwrap();

    // With unique exports, glob re-exports are fine
    assert!(mod_rs.contains("pub use button::*;"));
    assert!(mod_rs.contains("pub use input::*;"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_reexports_for_conflicts() {
    // When conflicts exist, should generate explicit re-exports
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Both export "Error"
    fs::write(
        src_dir.join("http.wj"),
        r#"
pub struct Request {}
pub struct Error {
    code: int,
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("db.wj"),
        r#"
pub struct Connection {}
pub struct Error {
    message: string,
}
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--module-file")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    let mod_rs = fs::read_to_string(output_dir.join("mod.rs")).unwrap();
    println!("Generated mod.rs:\n{}", mod_rs);

    // Should either:
    // A) Not use glob re-exports (no pub use *;)
    // B) Use explicit re-exports (pub use http::Request; etc.)
    // C) Add a comment warning about conflicts

    // For now, just check it compiles without ambiguity errors
    let compile_result = std::process::Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(output_dir.join("mod.rs"))
        .arg("--edition")
        .arg("2021")
        .arg("-L")
        .arg(&output_dir)
        .current_dir(&output_dir)
        .output();

    if let Ok(output) = compile_result {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should NOT have ambiguous re-export errors
        if stderr.contains("ambiguous") || stderr.contains("multiple applicable items") {
            panic!("Generated code has ambiguous re-exports:\n{}", stderr);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_module_reexports_unique() {
    // Test that nested modules handle re-exports correctly when no conflicts
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create nested directory
    let ui_dir = src_dir.join("ui");
    fs::create_dir(&ui_dir).unwrap();

    fs::write(ui_dir.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(ui_dir.join("input.wj"), "pub struct Input {}").unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--module-file")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Check nested mod.rs
    let nested_mod = fs::read_to_string(output_dir.join("ui/mod.rs")).unwrap();
    println!("Nested ui/mod.rs:\n{}", nested_mod);

    // Should have unique re-exports (no conflicts)
    assert!(nested_mod.contains("pub use button::*;"));
    assert!(nested_mod.contains("pub use input::*;"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_module_conflicts() {
    // Test that nested modules also detect and handle conflicts
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create nested directory with conflicting exports
    let components_dir = src_dir.join("components");
    fs::create_dir(&components_dir).unwrap();

    // Both export "State"
    fs::write(
        components_dir.join("toggle.wj"),
        r#"
pub struct Toggle {}
pub struct State {
    enabled: bool,
}
"#,
    )
    .unwrap();

    fs::write(
        components_dir.join("slider.wj"),
        r#"
pub struct Slider {}
pub struct State {
    value: int,
}
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--module-file")
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj");

    assert!(status.success());

    // Check nested components/mod.rs
    let nested_mod = fs::read_to_string(output_dir.join("components/mod.rs")).unwrap();
    println!("Nested components/mod.rs:\n{}", nested_mod);

    // Should NOT have glob re-exports due to conflict
    let has_glob_reexports =
        nested_mod.contains("pub use toggle::*;") && nested_mod.contains("pub use slider::*;");

    if has_glob_reexports {
        // If it has glob re-exports, verify it would cause an error
        panic!("Nested module should also detect and prevent ambiguous re-exports");
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_warning_for_potential_conflicts() {
    // Compiler should at least warn about potential conflicts
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    // Create files with common type names that might conflict
    fs::write(src_dir.join("types.wj"), "pub struct Result {}").unwrap();
    fs::write(src_dir.join("errors.wj"), "pub struct Result {}").unwrap();

    let output_dir = temp_dir.path().join("out");
    fs::create_dir(&output_dir).unwrap();

    // Run compilation and capture output
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--module-file")
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Stdout:\n{}", stdout);
    println!("Stderr:\n{}", stderr);

    // Compiler should either:
    // 1. Warn about the conflict
    // 2. Automatically handle it (no glob re-exports)

    // For now, just verify it succeeds and check the generated code
    assert!(output.status.success());

    let mod_rs = fs::read_to_string(output_dir.join("mod.rs")).unwrap();

    // If it uses glob re-exports, it will cause problems
    let uses_glob_reexports =
        mod_rs.contains("pub use types::*;") && mod_rs.contains("pub use errors::*;");

    if uses_glob_reexports {
        println!("⚠️  Warning: Generated code may have ambiguous re-exports");
    }
}
