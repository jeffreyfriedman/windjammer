//! TDD Tests for Trait Bound Inference
//!
//! These tests verify that the compiler correctly infers trait bounds
//! for generic type parameters by compiling Windjammer source and
//! checking the generated Rust code.

use std::process::Command;

/// Helper to compile a Windjammer snippet and get the generated Rust
fn compile_wj(source: &str, test_name: &str) -> Result<String, String> {
    // Write source to temp file with unique name
    let temp_dir = std::env::temp_dir();
    let wj_path = temp_dir.join(format!("{}.wj", test_name));
    let out_dir = temp_dir.join(format!("{}_out", test_name));

    std::fs::write(&wj_path, source).map_err(|e| e.to_string())?;

    // Determine the correct wj binary path based on test mode
    let wj_binary = if cfg!(debug_assertions) {
        // In debug mode, look for debug binary
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join("wj")
    } else {
        // In release mode, look for release binary
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("wj")
    };

    // If binary doesn't exist, build it first
    if !wj_binary.exists() {
        let build_mode = if cfg!(debug_assertions) { "debug" } else { "release" };
        let build_args = if cfg!(debug_assertions) {
            vec!["build", "--bin", "wj"]
        } else {
            vec!["build", "--release", "--bin", "wj"]
        };
        
        let build_output = Command::new("cargo")
            .args(&build_args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .map_err(|e| format!("Failed to build wj binary ({}): {}", build_mode, e))?;
            
        if !build_output.status.success() {
            return Err(format!(
                "Failed to build wj binary ({}):\n{}",
                build_mode,
                String::from_utf8_lossy(&build_output.stderr)
            ));
        }
    }

    // Compile with wj using the binary directly
    let output = Command::new(&wj_binary)
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| format!("Failed to execute wj: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    // Read generated Rust
    let rs_path = out_dir.join(format!("{}.rs", test_name));
    std::fs::read_to_string(&rs_path).map_err(|e| e.to_string())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_display_trait_inferred() {
    let source = r#"
fn print_item<T>(item: T) {
    println!("{}", item)
}

fn main() {
    print_item(42)
}
"#;

    let generated = compile_wj(source, "display_test").expect("Compilation failed");

    // Check that Display bound is inferred
    assert!(
        generated.contains("T: Display") || generated.contains("T: std::fmt::Display"),
        "Expected Display bound in:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_debug_trait_inferred() {
    let source = r#"
fn debug_item<T>(item: T) {
    println!("{:?}", item)
}

fn main() {
    debug_item(42)
}
"#;

    let generated = compile_wj(source, "debug_test").expect("Compilation failed");

    // Check that Debug bound is inferred
    assert!(
        generated.contains("T: Debug") || generated.contains("T: std::fmt::Debug"),
        "Expected Debug bound in:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_clone_trait_inferred() {
    let source = r#"
fn dup<T>(item: T) -> T {
    item.clone()
}

fn main() {
    let x = dup(42)
    println!("{}", x)
}
"#;

    let generated = compile_wj(source, "clone_test").expect("Compilation failed");

    // Check that Clone bound is inferred
    assert!(
        generated.contains("T: Clone") || generated.contains("Clone"),
        "Expected Clone bound in:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multiple_bounds_inferred() {
    let source = r#"
fn clone_and_print<T>(item: T) -> T {
    println!("{:?}", item)
    item.clone()
}

fn main() {
    let x = clone_and_print(42)
}
"#;

    let generated = compile_wj(source, "multi_bounds_test").expect("Compilation failed");

    // Check that both Clone and Debug bounds are inferred
    assert!(
        generated.contains("Clone") && generated.contains("Debug"),
        "Expected Clone + Debug bounds in:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_add_operator_trait_inferred() {
    let source = r#"
fn double<T>(x: T) -> T {
    x + x
}

fn main() {
    println!("{}", double(5))
}
"#;

    let generated = compile_wj(source, "add_operator_test").expect("Compilation failed");

    // Check that Add bound is inferred (with Output = T for same-type operands)
    assert!(
        generated.contains("Add<Output = T>") || generated.contains("Add"),
        "Expected Add bound in:\n{}",
        generated
    );
    // Should also have Copy since x is used twice
    assert!(
        generated.contains("Copy"),
        "Expected Copy bound in:\n{}",
        generated
    );
}
