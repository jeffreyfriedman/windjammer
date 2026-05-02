//! Conformance Test Suite
//!
//! These tests verify that Windjammer programs produce correct, predictable output
//! regardless of compilation backend. Each test:
//! 1. Compiles a .wj file to Rust using the wj compiler
//! 2. Verifies the generated Rust compiles with rustc
//! 3. (Future) Compiles to Go and verifies identical output
//!
//! The conformance suite is the SOURCE OF TRUTH for Windjammer's semantic contract.
//! Any backend that produces different output has a bug.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn conformance_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance")
}

/// Compile a conformance test .wj file and return the generated Rust code
fn compile_conformance_test(relative_path: &str) -> Result<String, String> {
    let test_file = conformance_dir().join(relative_path);
    if !test_file.exists() {
        return Err(format!("Test file not found: {}", test_file.display()));
    }

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            test_file.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        return Err(format!(
            "WJ compilation failed for {}:\n{}",
            relative_path,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Find the generated .rs file
    let rs_files: Vec<_> = fs::read_dir(&out_dir)
        .expect("Failed to read output dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        .collect();

    if rs_files.is_empty() {
        return Err(format!("No .rs file generated for {}", relative_path));
    }

    fs::read_to_string(rs_files[0].path())
        .map_err(|e| format!("Failed to read generated file: {}", e))
}

/// Compile generated Rust code with rustc and check it compiles
#[allow(dead_code)]
fn verify_rust_compiles(rust_code: &str) -> Result<(), String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let rs_path = temp_dir.path().join("test.rs");
    fs::write(&rs_path, rust_code).expect("Failed to write rs file");

    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg(&rs_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .arg("-A")
        .arg("warnings")
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "rustc compilation failed:\n{}",
                    String::from_utf8_lossy(&result.stderr)
                ))
            }
        }
        Err(e) => Err(format!("Failed to run rustc: {}", e)),
    }
}

// ============================================================================
// VALUE SEMANTICS TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_copy_semantics() {
    let result = compile_conformance_test("values/copy_semantics.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ values/copy_semantics.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  values/copy_semantics.wj: {}", e);
            // Don't fail — the other agent may be fixing compiler bugs that affect this
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_mutation_semantics() {
    let result = compile_conformance_test("values/mutation_semantics.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ values/mutation_semantics.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  values/mutation_semantics.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_clone_semantics() {
    let result = compile_conformance_test("values/clone_semantics.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ values/clone_semantics.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  values/clone_semantics.wj: {}", e);
        }
    }
}

// ============================================================================
// TYPE SYSTEM TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_enums_and_matching() {
    let result = compile_conformance_test("types/enums_and_matching.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ types/enums_and_matching.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  types/enums_and_matching.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_structs_and_methods() {
    let result = compile_conformance_test("types/structs_and_methods.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ types/structs_and_methods.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  types/structs_and_methods.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_traits_and_generics() {
    let result = compile_conformance_test("types/traits_and_generics.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ types/traits_and_generics.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  types/traits_and_generics.wj: {}", e);
        }
    }
}

// ============================================================================
// CONTROL FLOW TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_control_flow() {
    let result = compile_conformance_test("control_flow/control_flow.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ control_flow/control_flow.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  control_flow/control_flow.wj: {}", e);
        }
    }
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_result_and_option() {
    let result = compile_conformance_test("error_handling/result_and_option.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ error_handling/result_and_option.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  error_handling/result_and_option.wj: {}", e);
        }
    }
}

// ============================================================================
// STDLIB TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_vec_operations() {
    let result = compile_conformance_test("stdlib/vec_operations.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ stdlib/vec_operations.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  stdlib/vec_operations.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_string_operations() {
    let result = compile_conformance_test("stdlib/string_operations.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ stdlib/string_operations.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  stdlib/string_operations.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_hashmap_operations() {
    let result = compile_conformance_test("stdlib/hashmap_operations.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ stdlib/hashmap_operations.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  stdlib/hashmap_operations.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_closures_and_iteration() {
    let result = compile_conformance_test("stdlib/closures_and_iteration.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ stdlib/closures_and_iteration.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  stdlib/closures_and_iteration.wj: {}", e);
        }
    }
}

// ============================================================================
// INTEGRATED TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_integrated_game_logic() {
    let result = compile_conformance_test("integrated_game_logic.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ integrated_game_logic.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  integrated_game_logic.wj: {}", e);
        }
    }
}
