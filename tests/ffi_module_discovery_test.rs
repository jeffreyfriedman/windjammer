/// TDD Test: FFI modules in src/ffi/ should be auto-discovered and added to lib.rs
///
/// THE WINDJAMMER WAY: Hand-written Rust modules (like FFI) should be automatically
/// discovered and integrated into the generated lib.rs file.
///
/// BUG: lib.rs was missing `pub mod ffi;` even though src/ffi/renderer.rs exists
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ffi_module_auto_discovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();

    // Create a Windjammer project structure
    let src_wj = project_root.join("src_wj");
    std::fs::create_dir_all(&src_wj).unwrap();

    // Create a simple mod.wj
    std::fs::write(
        src_wj.join("mod.wj"),
        r#"
    pub mod math;
    "#,
    )
    .unwrap();

    // Create a math module
    std::fs::create_dir_all(src_wj.join("math")).unwrap();
    std::fs::write(
        src_wj.join("math/vec2.wj"),
        r#"
    pub struct Vec2 {
        pub x: f32,
        pub y: f32,
    }
    "#,
    )
    .unwrap();

    std::fs::write(src_wj.join("math/mod.wj"), "pub use vec2::Vec2;").unwrap();

    // THE WINDJAMMER WAY: Create hand-written FFI module in src/ffi/
    // This should be auto-discovered!
    let src_ffi = project_root.join("src/ffi");
    std::fs::create_dir_all(&src_ffi).unwrap();

    // FFI module must have mod.rs to be recognized as a module
    std::fs::write(
        src_ffi.join("mod.rs"),
        r#"
    // Hand-written Rust FFI module
    pub mod renderer;
    pub use renderer::*;
    "#,
    )
    .unwrap();

    std::fs::write(
        src_ffi.join("renderer.rs"),
        r#"
    // Hand-written Rust FFI module
    pub fn render_clear(r: f32, g: f32, b: f32, a: f32) {
        // Stub implementation
    }
    "#,
    )
    .unwrap();

    // Compile the project
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output_dir = project_root.join("output");

    let compile_result = Command::new(&wj_binary)
        .args([
            "build",
            src_wj.join("mod.wj").to_str().unwrap(),
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

    // Verify it compiles with rustc
    let rustc_result = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(output_dir.join("lib.rs"))
        .arg("--out-dir")
        .arg(&output_dir)
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to run rustc");

    assert!(
        rustc_result.status.success(),
        "Generated code should compile with rustc!\nrustc stderr:\n{}",
        String::from_utf8_lossy(&rustc_result.stderr)
    );
}
