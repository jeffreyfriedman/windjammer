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

/// TDD: Fix pub use module path bug
/// BUG: Generates `pub use voxel_grid::VoxelGrid` instead of `pub use self::voxel_grid::VoxelGrid`
/// FIX: Add `self::` prefix for module-relative pub use
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_pub_use_generates_self_prefix() {
    let wj_source = r#"
pub mod inner_module

pub use inner_module::MyType

pub mod inner_module {
    pub struct MyType {
        pub value: i32,
    }
}

fn main() {
    let t = MyType { value: 42 }
    println!("{}", t.value)
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, wj_source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            "--target",
            "rust",
            "--no-cargo",
            test_file.to_str().unwrap(),
            "--output",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rust_file = temp_dir.path().join("test.rs");
    let rust_code = fs::read_to_string(&rust_file).expect("Generated Rust file not found");

    println!("Generated Rust code:\n{}", rust_code);

    // Should use self:: prefix for module-relative imports
    assert!(
        rust_code.contains("pub use self::inner_module::MyType")
            || rust_code.contains("pub use crate::inner_module::MyType"),
        "pub use should have self:: or crate:: prefix, got:\n{}",
        rust_code
    );

    assert!(
        !rust_code.contains("pub use inner_module::VoxelGrid;")
            || rust_code.contains("self::")
            || rust_code.contains("crate::"),
        "Bare module paths in pub use should be qualified"
    );
}
