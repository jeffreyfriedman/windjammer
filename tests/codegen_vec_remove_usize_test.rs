#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

use std::fs;
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_remove_usize_no_ref() {
    let wj_binary = env!("CARGO_BIN_EXE_wj");

    let test_dir = tempfile::TempDir::new().expect("tempdir");

    // Test that Vec.remove(usize_var) does NOT add &
    // Vec.remove takes usize by value, not by reference
    let test_content = r#"
fn remove_at_sparse_index(mut items: Vec<i32>, sparse_index: int) -> i32 {
    let sparse_idx_usize: usize = sparse_index as usize;
    items.remove(sparse_idx_usize)
}

fn main() {
    let mut items = vec![10, 20, 30];
    let removed = remove_at_sparse_index(items, 1);
    println!("Removed: {}", removed);
}
"#;

    let test_file = test_dir.path().join("vec_remove_usize.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(wj_binary)
        .current_dir(test_dir.path())
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.path().join("build").join("vec_remove_usize.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // Vec.remove takes usize by value — no `&`. Tip may re-state `as usize`.
    assert!(
        rust_code.contains("items.remove(sparse_idx_usize)")
            || rust_code.contains("items.remove((sparse_idx_usize) as usize)"),
        "Expected NO auto-ref for Vec.remove(usize).\nGenerated code:\n{}",
        rust_code
    );

    // Should NOT contain the incorrect version
    assert!(
        !rust_code.contains("items.remove(&sparse_idx_usize)")
            && !rust_code.contains("items.remove(&(sparse_idx_usize)"),
        "Should NOT add & to usize for Vec.remove.\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles
    let compile_output = Command::new("rustc")
        .current_dir(test_dir.path().join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg("vec_remove_usize.rs")
        .output()
        .expect("Failed to run rustc");

    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    assert!(
        compile_output.status.success(),
        "Expected generated code to compile.\nRustc errors:\n{}",
        compile_stderr
    );
}
