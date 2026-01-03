/// TDD Test: FFI Module Handling
///
/// Tests that Windjammer can handle external Rust modules (FFI) alongside generated code.
///
/// The pattern:
/// 1. Windjammer generates .rs files from .wj files
/// 2. User provides hand-written Rust FFI code in src/ffi.rs or src/ffi/mod.rs
/// 3. Generated lib.rs should declare `pub mod ffi;` if src/ffi exists
/// 4. Windjammer code can `use crate::ffi` to access FFI functions
use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ffi_module_declaration() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src_wj");
    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // Create a Windjammer file that uses FFI
    let game_wj = r#"
use crate::ffi

extern fn get_window_width() -> int

pub fn check_size() -> bool {
    let width = ffi::get_window_width()
    width > 800
}
"#;
    fs::write(src_dir.join("game.wj"), game_wj).unwrap();

    // Create a second file to make it a multi-file project (so lib.rs is generated)
    let utils_wj = r#"
pub fn helper() -> int {
    42
}
"#;
    fs::write(src_dir.join("utils.wj"), utils_wj).unwrap();

    // Create hand-written FFI module in project root (alongside src_wj)
    // THE WINDJAMMER WAY: FFI modules live in project root and are auto-discovered!
    let ffi_rs = r#"
// Hand-written Rust FFI code
// THE WINDJAMMER WAY: Provide safe wrappers for FFI functions
pub fn get_window_width() -> i64 {
    // In a real implementation, this would call actual FFI
    // For test purposes, just return a dummy value
    1024
}
"#;
    let project_root = temp_dir.path();
    fs::write(project_root.join("ffi.rs"), ffi_rs).unwrap();

    // Compile the Windjammer project
    let result =
        windjammer::build_project(&src_dir, &output_dir, windjammer::CompilationTarget::Rust);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Check that lib.rs was generated and declares ffi module
    let lib_rs_path = output_dir.join("lib.rs");
    assert!(lib_rs_path.exists(), "lib.rs should be generated");

    let lib_rs = fs::read_to_string(&lib_rs_path).unwrap();
    println!("Generated lib.rs:\n{}", lib_rs);

    // THE WINDJAMMER WAY: If src/ffi.rs exists, lib.rs should declare it
    // This allows hand-written Rust FFI code to coexist with generated code
    assert!(
        lib_rs.contains("pub mod ffi;"),
        "lib.rs should declare ffi module when ffi.rs exists"
    );

    // Check that game.rs uses the ffi module correctly
    let game_rs = fs::read_to_string(output_dir.join("game.rs")).unwrap();
    assert!(
        game_rs.contains("use crate::ffi"),
        "Generated code should import ffi module"
    );

    // Verify it compiles with rustc
    let cargo_output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&output_dir)
        .output()
        .expect("Failed to run cargo build");

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        println!("Cargo build failed:\n{}", stderr);

        // Check for specific FFI errors
        if stderr.contains("unresolved import `crate::ffi`") {
            panic!("FFI module not declared in lib.rs");
        }

        panic!("Generated Rust should compile");
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ffi_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src_wj");
    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // Create Windjammer file
    let game_wj = r#"
use crate::ffi

pub fn init() {
    ffi::initialize()
}
"#;
    fs::write(src_dir.join("game.wj"), game_wj).unwrap();

    // Create a second file to ensure multi-file project
    let utils_wj = r#"
pub fn helper() -> int {
    42
}
"#;
    fs::write(src_dir.join("utils.wj"), utils_wj).unwrap();

    // Create FFI as a subdirectory with mod.rs in project root
    // THE WINDJAMMER WAY: Hand-written modules live in project root!
    let project_root = temp_dir.path();
    let ffi_dir = project_root.join("ffi");
    fs::create_dir_all(&ffi_dir).unwrap();

    let ffi_mod_rs = r#"
// FFI module
pub fn initialize() {
    println!("Initialized");
}
"#;
    fs::write(ffi_dir.join("mod.rs"), ffi_mod_rs).unwrap();

    // Compile
    let result =
        windjammer::build_project(&src_dir, &output_dir, windjammer::CompilationTarget::Rust);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Check lib.rs declares ffi module
    let lib_rs = fs::read_to_string(output_dir.join("lib.rs")).unwrap();
    println!("Generated lib.rs:\n{}", lib_rs);

    assert!(
        lib_rs.contains("pub mod ffi;"),
        "lib.rs should declare ffi module when ffi/mod.rs exists"
    );

    // Check that generated game.rs imports ffi
    let game_rs = fs::read_to_string(output_dir.join("game.rs")).unwrap();
    assert!(
        game_rs.contains("use crate::ffi"),
        "Generated code should import ffi module"
    );

    // THE WINDJAMMER WAY: FFI modules are discovered and integrated automatically!
    // No manual module declarations needed - compiler does the work
}
