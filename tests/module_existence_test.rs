// TDD: Test that compiler doesn't generate declarations for non-existent modules
// Write tests FIRST, then fix the compiler

use std::fs;
use tempfile::TempDir;

#[test]
fn test_module_file_skips_missing_modules() {
    // Create a directory with some .wj files
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    
    // Create actual .wj files
    fs::write(src_dir.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(src_dir.join("input.wj"), "pub struct Input {}").unwrap();
    
    // Compile with --module-file
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
    
    // Read generated mod.rs
    let mod_rs = fs::read_to_string(output_dir.join("mod.rs"))
        .expect("mod.rs should be generated");
    
    println!("Generated mod.rs:\n{}", mod_rs);
    
    // Verify only existing modules are declared
    assert!(mod_rs.contains("pub mod button;"), "button.rs exists, should be declared");
    assert!(mod_rs.contains("pub mod input;"), "input.rs exists, should be declared");
    
    // Verify mod.rs itself is NOT declared (would cause recursion)
    assert!(!mod_rs.contains("pub mod mod;"), "mod.rs should NOT be declared");
    
    // Verify lib.rs is NOT declared if it doesn't exist
    assert!(!mod_rs.contains("pub mod lib;"), "lib.rs doesn't exist, should NOT be declared");
}

#[test]
fn test_generated_rust_files_excluded_from_module_list() {
    // When compiling a directory, generated .rs files should NOT be included in mod.rs
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    
    // Create .wj files
    fs::write(src_dir.join("component.wj"), "pub struct Component {}").unwrap();
    
    // Compile to output dir
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
    
    // Verify component.rs was generated
    assert!(
        output_dir.join("component.rs").exists(),
        "component.rs should be generated"
    );
    
    let mod_rs = fs::read_to_string(output_dir.join("mod.rs")).unwrap();
    
    println!("Generated mod.rs:\n{}", mod_rs);
    
    // Should declare component module
    assert!(mod_rs.contains("pub mod component;"));
    
    // Should NOT declare mod itself
    assert!(!mod_rs.contains("pub mod mod;"));
    
    // Count how many module declarations (should be exactly 1: component)
    let mod_count = mod_rs.matches("pub mod ").count();
    assert_eq!(
        mod_count, 1,
        "Should have exactly 1 module declaration (component), got {}",
        mod_count
    );
}

#[test]
fn test_nested_modules_only_declare_existing_files() {
    // Test nested directory structure
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    
    // Create nested directory
    let components_dir = src_dir.join("components");
    fs::create_dir(&components_dir).unwrap();
    
    // Create some files in nested dir
    fs::write(components_dir.join("button.wj"), "pub struct Button {}").unwrap();
    fs::write(components_dir.join("input.wj"), "pub struct Input {}").unwrap();
    
    // Also create a file at root
    fs::write(src_dir.join("app.wj"), "pub struct App {}").unwrap();
    
    // Compile
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
    
    // Check root mod.rs
    let root_mod = fs::read_to_string(output_dir.join("mod.rs")).unwrap();
    println!("Root mod.rs:\n{}", root_mod);
    
    assert!(root_mod.contains("pub mod app;"), "app should be declared");
    assert!(root_mod.contains("pub mod components;"), "components subdir should be declared");
    
    // Check nested mod.rs
    let components_mod = fs::read_to_string(output_dir.join("components/mod.rs")).unwrap();
    println!("\nComponents mod.rs:\n{}", components_mod);
    
    assert!(components_mod.contains("pub mod button;"), "button should be declared");
    assert!(components_mod.contains("pub mod input;"), "input should be declared");
    assert!(!components_mod.contains("pub mod mod;"), "mod.rs should NOT declare itself");
}

#[test]
fn test_empty_directory_generates_empty_mod_file() {
    // Edge case: directory with no .wj files
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    
    // Create an empty subdirectory
    let empty_dir = src_dir.join("empty");
    fs::create_dir(&empty_dir).unwrap();
    
    // Create one .wj file at root so compilation proceeds
    fs::write(src_dir.join("main.wj"), "fn main() {}").unwrap();
    
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
    
    // Check if empty/mod.rs was created
    let empty_mod_path = output_dir.join("empty/mod.rs");
    if empty_mod_path.exists() {
        let empty_mod = fs::read_to_string(empty_mod_path).unwrap();
        println!("Empty mod.rs:\n{}", empty_mod);
        
        // Should not declare any modules (empty directory)
        let mod_count = empty_mod.matches("pub mod ").count();
        assert_eq!(
            mod_count, 0,
            "Empty directory should have no module declarations, got {}",
            mod_count
        );
    }
}

#[test]
fn test_cargo_toml_excluded_from_module_list() {
    // Ensure Cargo.toml and other non-Rust files aren't declared as modules
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    
    // Create multiple modules so mod.rs will be generated
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
    
    // Manually create a Cargo.toml in output (simulating what --library mode does)
    fs::write(output_dir.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
    
    let mod_rs = fs::read_to_string(output_dir.join("mod.rs"))
        .expect("mod.rs should be generated");
    
    // Should NOT try to declare Cargo.toml as a module
    assert!(!mod_rs.contains("Cargo"), "Cargo.toml should not appear in mod.rs");
    assert!(!mod_rs.contains("toml"), "Cargo.toml should not appear in mod.rs");
    
    // Should declare the actual modules
    assert!(mod_rs.contains("pub mod button;"));
    assert!(mod_rs.contains("pub mod input;"));
}

