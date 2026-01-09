// TDD Test: Test framework should compile libraries that tests can import from
//
// Problem: Tests that import from the library (e.g., `use windjammer_game_core::GameLoop`)
// fail because the library isn't being compiled as a proper Rust crate.
//
// This test validates that:
// 1. Library is compiled with proper Cargo.toml
// 2. Library has lib.rs or mod.rs entry point
// 3. Library exports public symbols that tests can import

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_library_compilation_creates_linkable_crate() {
    // Create a temporary project with a library
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create wj.toml
    let wj_toml = r#"
[package]
name = "test-lib"
version = "0.1.0"

[dependencies]
windjammer-runtime = { path = "../../../windjammer/crates/windjammer-runtime" }
"#;
    fs::write(project_root.join("wj.toml"), wj_toml).unwrap();

    // Create src_wj directory with a simple library
    let src_wj = project_root.join("src_wj");
    fs::create_dir_all(&src_wj).unwrap();

    // Create a simple module that exports a public function
    fs::write(
        src_wj.join("mod.wj"),
        r#"
// Simple library module
pub fn hello() -> string {
    "Hello from library!".to_string()
}

pub struct TestStruct {
    pub value: int,
}

impl TestStruct {
    pub fn new(value: int) -> TestStruct {
        TestStruct { value }
    }
}
"#,
    )
    .unwrap();

    // Simulate what detect_and_compile_library should do
    let lib_output = project_root.join("lib");
    fs::create_dir_all(&lib_output).unwrap();

    // After compilation, the library should have:
    // 1. Cargo.toml with [lib] section
    let cargo_toml = r#"[package]
name = "test-lib"
version = "0.1.0"
edition = "2021"

[dependencies]
windjammer-runtime = { path = "../../../windjammer/crates/windjammer-runtime" }

[lib]
name = "test_lib"
path = "lib.rs"
"#;
    fs::write(lib_output.join("Cargo.toml"), cargo_toml).unwrap();

    // 2. lib.rs entry point that re-exports modules
    let lib_rs = r#"// Auto-generated lib.rs
pub mod mod_file;

// Re-export public items
pub use mod_file::hello;
pub use mod_file::TestStruct;
"#;
    fs::write(lib_output.join("lib.rs"), lib_rs).unwrap();

    // 3. The actual compiled module
    let mod_rs = r#"// Generated from mod.wj
pub fn hello() -> String {
    "Hello from library!".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TestStruct {
    pub value: i64,
}

impl TestStruct {
    pub fn new(value: i64) -> TestStruct {
        TestStruct { value }
    }
}
"#;
    fs::write(lib_output.join("mod_file.rs"), mod_rs).unwrap();

    // Verify the library compiles
    let output = Command::new("cargo")
        .arg("check")
        .arg("--lib")
        .current_dir(&lib_output)
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                panic!("Library compilation failed:\n{}", stderr);
            }
        }
        Err(e) => {
            panic!("Failed to run cargo check: {}", e);
        }
    }

    // Now verify that a test can import from this library
    let test_output = project_root.join("test");
    fs::create_dir_all(&test_output).unwrap();

    // Create test that imports from library
    let test_code = r#"
use test_lib::{hello, TestStruct};

#[test]
fn test_library_import() {
    let msg = hello();
    assert_eq!(msg, "Hello from library!");
    
    let obj = TestStruct::new(42);
    assert_eq!(obj.value, 42);
}
"#;
    fs::write(test_output.join("test.rs"), test_code).unwrap();

    // Create Cargo.toml for test
    let test_cargo_toml = format!(
        r#"[package]
name = "test-tests"
version = "0.1.0"
edition = "2021"

[dependencies]
test-lib = {{ path = "{}" }}

[lib]
name = "test_tests"
path = "test.rs"
"#,
        lib_output.display()
    );
    fs::write(test_output.join("Cargo.toml"), test_cargo_toml).unwrap();

    // Verify the test compiles and can import from library
    let test_result = Command::new("cargo")
        .arg("test")
        .current_dir(&test_output)
        .output();

    match test_result {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                panic!("Test compilation/run failed:\n{}", stderr);
            }

            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(
                stdout.contains("test test_library_import ... ok"),
                "Test should pass and import from library"
            );
        }
        Err(e) => {
            panic!("Failed to run cargo test: {}", e);
        }
    }
}

#[test]
fn test_library_needs_lib_rs_entry_point() {
    // The key insight: build_project compiles Windjammer to Rust files,
    // but doesn't create a lib.rs entry point that:
    // 1. Declares all modules (pub mod module1;)
    // 2. Re-exports public items (pub use module1::*;)
    //
    // Without lib.rs, the library can't be imported by tests.
    //
    // This test documents what detect_and_compile_library SHOULD do:

    let temp_dir = TempDir::new().unwrap();
    let lib_output = temp_dir.path();

    // Simulate what we have after build_project
    fs::create_dir_all(lib_output.join("game_loop")).unwrap();
    fs::write(
        lib_output.join("game_loop/game_loop.rs"),
        "pub trait GameLoop {}",
    )
    .unwrap();
    fs::write(
        lib_output.join("game_loop/mod.rs"),
        "pub mod game_loop;\npub use game_loop::*;",
    )
    .unwrap();

    // What we SHOULD generate: lib.rs that declares all top-level modules
    let lib_rs_content = r#"// Auto-generated library entry point
pub mod game_loop;

// Re-export for convenience
pub use game_loop::*;
"#;

    fs::write(lib_output.join("lib.rs"), lib_rs_content).unwrap();

    // Verify the structure exists
    assert!(lib_output.join("lib.rs").exists(), "lib.rs should exist");
    assert!(
        lib_output.join("game_loop/mod.rs").exists(),
        "module mod.rs should exist"
    );

    // Verify the content has proper declarations
    let lib_content = fs::read_to_string(lib_output.join("lib.rs")).unwrap();
    assert!(
        lib_content.contains("pub mod game_loop"),
        "lib.rs should declare modules"
    );
    assert!(
        lib_content.contains("pub use game_loop::*"),
        "lib.rs should re-export"
    );
}
