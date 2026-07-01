#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

// Integration test for static method inference bug
// Verifies that constructors like new(), from_*(), zero() don't get &self

use std::path::{Path, PathBuf};

fn find_generated_rs(dir: &Path, filename: &str) -> Result<String, std::io::Error> {
    let flat = dir.join(filename);
    if flat.exists() {
        return std::fs::read_to_string(&flat);
    }
    fn walk(dir: &Path, target: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.file_name().map_or(false, |n| n == target) {
                return Some(path);
            }
            if path.is_dir() {
                if let Some(found) = walk(&path, target) {
                    return Some(found);
                }
            }
        }
        None
    }
    match walk(dir, filename) {
        Some(p) => std::fs::read_to_string(&p),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} not found under {}", filename, dir.display()),
        )),
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_static_method_inference() {
    let out_tmp = tempfile::tempdir().expect("tempdir");
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("static_method_inference_test.wj");

    windjammer::build_project(
        &wj_path,
        out_tmp.path(),
        windjammer::CompilationTarget::Rust,
        false,
    )
    .expect("Failed to run windjammer compiler");

    let generated_code = find_generated_rs(out_tmp.path(), "static_method_inference_test.rs")
        .expect("Failed to find generated static_method_inference_test.rs");

    // Verify static methods do NOT have &self
    assert!(
        generated_code.contains("pub fn new(x: f32, y: f32) -> Point"),
        "new() should not have &self parameter"
    );

    assert!(
        generated_code.contains("pub fn from_coords(x: f32, y: f32) -> Point"),
        "from_coords() should not have &self parameter"
    );

    assert!(
        generated_code.contains("pub fn zero() -> Point"),
        "zero() should not have &self parameter"
    );

    // Verify instance method DOES have &self
    assert!(
        generated_code.contains("pub fn distance(&self, other: Point) -> f32"),
        "distance() should have &self parameter"
    );

    // THE BUG: Grid::new() should NOT have &self even though it uses field names in struct literal
    assert!(
        generated_code.contains("pub fn new(width: i32, height: i32) -> Grid"),
        "Grid::new() should not have &self parameter (this is the bug!)"
    );

    // Note: Skipping rustc verification because generated code includes test decorators
    // and windjammer_runtime imports that require cargo build environment.
    // The function signature checks above are sufficient to verify correctness.
}
