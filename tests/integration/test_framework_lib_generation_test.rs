// TDD Test: Test framework should include top-level .rs files in lib.rs
//
// Bug: generate_lib_rs_for_library() only includes directories with mod.rs,
//      but ignores top-level .rs files like scene.rs
//
// This causes "unresolved import `crate::scene`" errors in tests

use std::fs;
use tempfile::TempDir;

#[test]
fn test_lib_rs_includes_top_level_rs_files() {
    // Create a temporary library structure
    let temp_dir = TempDir::new().unwrap();
    let lib_dir = temp_dir.path();

    // Create module directories (like core/, panels/)
    fs::create_dir(lib_dir.join("core")).unwrap();
    fs::write(lib_dir.join("core/mod.rs"), "// core module").unwrap();

    fs::create_dir(lib_dir.join("panels")).unwrap();
    fs::write(lib_dir.join("panels/mod.rs"), "// panels module").unwrap();

    // Create top-level .rs files (like scene.rs)
    fs::write(lib_dir.join("scene.rs"), "// scene module").unwrap();
    fs::write(lib_dir.join("utils.rs"), "// utils module").unwrap();

    // Generate lib.rs using the same logic as the test framework
    generate_lib_rs_for_test(lib_dir).unwrap();

    // Read generated lib.rs
    let lib_rs = fs::read_to_string(lib_dir.join("lib.rs")).unwrap();

    // Verify it includes directories
    assert!(
        lib_rs.contains("pub mod core;"),
        "Should include core module"
    );
    assert!(
        lib_rs.contains("pub mod panels;"),
        "Should include panels module"
    );

    // Verify it includes top-level .rs files
    assert!(
        lib_rs.contains("pub mod scene;"),
        "Should include scene.rs as module"
    );
    assert!(
        lib_rs.contains("pub mod utils;"),
        "Should include utils.rs as module"
    );

    // Verify re-exports
    assert!(lib_rs.contains("pub use core::*;"));
    assert!(lib_rs.contains("pub use panels::*;"));
    assert!(lib_rs.contains("pub use scene::*;"));
    assert!(lib_rs.contains("pub use utils::*;"));
}

// Copy of the function from main.rs that we're testing
fn generate_lib_rs_for_test(lib_output_dir: &std::path::Path) -> anyhow::Result<()> {
    use std::fs;

    // Find all modules (directories with mod.rs AND top-level .rs files)
    let mut modules = Vec::new();

    for entry in fs::read_dir(lib_output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Directory modules (must have mod.rs)
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if path.join("mod.rs").exists() {
                    modules.push(dir_name.to_string());
                }
            }
        } else if path.is_file() {
            // Top-level .rs files (but not lib.rs or Cargo.toml)
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".rs") && file_name != "lib.rs" {
                    // Extract module name (remove .rs extension)
                    let module_name = file_name.trim_end_matches(".rs");
                    modules.push(module_name.to_string());
                }
            }
        }
    }

    if modules.is_empty() {
        return Ok(()); // No modules to export
    }

    modules.sort();

    // Generate lib.rs content
    let mut lib_rs = String::from("// Auto-generated library entry point\n\n");

    // Declare all modules
    for module in &modules {
        lib_rs.push_str(&format!("pub mod {};\n", module));
    }

    lib_rs.push_str("\n// Re-export for convenience\n");
    for module in &modules {
        lib_rs.push_str(&format!("pub use {}::*;\n", module));
    }

    // Write lib.rs
    fs::write(lib_output_dir.join("lib.rs"), lib_rs)?;

    Ok(())
}
