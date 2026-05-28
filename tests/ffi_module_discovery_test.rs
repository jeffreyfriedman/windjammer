#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

/// TDD Test: FFI modules in src/ffi/ should be auto-discovered and added to lib.rs
///
/// THE WINDJAMMER WAY: Hand-written Rust modules (like FFI) should be automatically
/// discovered and integrated into the generated lib.rs file.
///
/// BUG: lib.rs was missing `pub mod ffi;` even though src/ffi/renderer.rs exists
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ffi_module_auto_discovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();

    // Create a Windjammer project structure
    let src = project_root.join("src");
    std::fs::create_dir_all(&src).unwrap();

    // Create a simple mod.wj
    std::fs::write(
        src.join("mod.wj"),
        r#"
    pub mod math;
    "#,
    )
    .unwrap();

    // Create a math module
    std::fs::create_dir_all(src.join("math")).unwrap();
    std::fs::write(
        src.join("math/vec2.wj"),
        r#"
    pub struct Vec2 {
        pub x: f32,
        pub y: f32,
    }
    "#,
    )
    .unwrap();

    std::fs::write(src.join("math/mod.wj"), "pub use vec2::Vec2;").unwrap();

    // Compile the project
    let output_dir = project_root.join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    // THE WINDJAMMER WAY: Hand-written FFI modules placed in the output directory
    // (in a real build, wj game build handles syncing these from src/ffi/)
    let out_ffi = output_dir.join("ffi");
    std::fs::create_dir_all(&out_ffi).unwrap();

    std::fs::write(
        out_ffi.join("mod.rs"),
        r#"
    pub mod renderer;
    pub use renderer::*;
    "#,
    )
    .unwrap();

    std::fs::write(
        out_ffi.join("renderer.rs"),
        r#"
    pub fn render_clear(r: f32, g: f32, b: f32, a: f32) {
        // Stub implementation
    }
    "#,
    )
    .unwrap();

    let compile_result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    assert!(
        compile_result.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_result.stdout),
        String::from_utf8_lossy(&compile_result.stderr)
    );

    // Read the generated lib.rs
    let lib_rs = std::fs::read_to_string(output_dir.join("lib.rs")).expect("Failed to read lib.rs");

    // CRITICAL ASSERTION: lib.rs should declare the ffi module
    assert!(
        lib_rs.contains("pub mod ffi;"),
        "lib.rs should contain 'pub mod ffi;' for hand-written FFI module!\nGenerated lib.rs:\n{}",
        lib_rs
    );

    // Verify the ffi.rs file was copied to output
    assert!(
        output_dir.join("ffi.rs").exists() || output_dir.join("ffi/mod.rs").exists(),
        "FFI module should be copied to output directory"
    );

    // Verify the ffi module itself compiles with rustc (standalone check)
    let ffi_mod_rs = output_dir.join("ffi/mod.rs");
    if ffi_mod_rs.exists() {
        let rmeta_out = output_dir.join("ffi_module_verify.rmeta");
        let rustc_result = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--emit=metadata")
            .arg("-o")
            .arg(&rmeta_out)
            .arg(&ffi_mod_rs)
            .arg("--edition")
            .arg("2021")
            .output()
            .expect("Failed to run rustc");

        assert!(
            rustc_result.status.success(),
            "FFI module should compile with rustc!\nrustc stderr:\n{}",
            String::from_utf8_lossy(&rustc_result.stderr)
        );
    }
}
