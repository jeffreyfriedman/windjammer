#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_mut_on_compound_assignment() {
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_auto_mut_compound");

    fs::create_dir_all(&test_dir).unwrap();

    // Test that `let mut` with compound assignment generates correct Rust code.
    // Immutable-by-default: users must explicitly write `let mut` for mutable bindings.
    let test_content = r#"
fn count_items(items: Vec<i32>) -> i32 {
    let mut total = 0;
    for item in items {
        total += item;
    }
    total
}

fn main() {
    let result = count_items(vec![1, 2, 3]);
}
"#;

    let test_file = test_dir.join("compound.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("--no-cargo")
        .arg("compound.wj")
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    // Find the generated Rust file (compiler may nest paths differently)
    let rust_file = test_dir.join("build").join("compound.rs");
    let rust_file = if rust_file.exists() {
        rust_file
    } else {
        // Search for compound.rs anywhere in the build directory
        fn find_rs(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.file_name().map(|f| f == name).unwrap_or(false) {
                        return Some(p);
                    }
                    if p.is_dir() {
                        if let Some(found) = find_rs(&p, name) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }
        find_rs(&test_dir.join("build"), "compound.rs")
            .unwrap_or_else(|| panic!("Could not find compound.rs in {:?}/build/", test_dir))
    };

    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should have `let mut total` with integer suffix (from int inference)
    assert!(
        rust_code.contains("let mut total = 0_i32;") || rust_code.contains("let mut total = 0;"),
        "Expected `let mut` to generate `let mut` in Rust output.\nGenerated code:\n{}",
        rust_code
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}
