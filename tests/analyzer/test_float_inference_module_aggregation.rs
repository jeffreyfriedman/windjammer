//! TDD Test: Module-level float type conflict (PROPER FIX)
//!
//! Problem: Individual .wj files compile successfully, but when aggregated in a module,
//! float type conflicts appear. Example:
//!   - companion.wj with update(dt: f32) works alone
//!   - kestrel.wj calling companion.update(dt) works alone
//!   - mod.wj aggregating both fails: Type conflict at seq_id=35, 56:33
//!
//! Root cause: Float inference state may be shared or not properly reset between
//! module files during aggregation.
//!
//! Goal: Module aggregation doesn't create spurious type conflicts.
//!
//! Investigation Tasks (when bug is reproduced):
//! 1. Add debug logging in float_inference.rs: eprintln!("DEBUG: Type conflict at expr seq_id={} ({}:{})", ...)
//! 2. Check inference state: Is expr_id_cache/next_seq_id shared between module files?
//! 3. Fix reset logic: Clear FloatInference state between files if needed
//! 4. Verify: test passes + breach-protocol builds

use std::fs;
use tempfile::TempDir;
use windjammer::{build_project, build_project_ext, CompilationTarget};

#[test]
fn test_two_files_with_f32_params_no_conflict_compiler() {
    // Test using windjammer::build_project (compiler path)
    let temp_dir = TempDir::new().unwrap();
    let pkg_dir = temp_dir.path().join("pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    // File 1: Base type with f32 method (matches breach-protocol companion pattern)
    fs::write(
        pkg_dir.join("base.wj"),
        r#"
pub struct Base {
    pub value: f32,
}

impl Base {
    pub fn update(self, dt: f32) {
        if self.value > 0.0 {
            self.value = self.value - dt
        }
    }
}
"#,
    )
    .unwrap();

    // File 2: Wrapper that calls base
    fs::write(
        pkg_dir.join("wrapper.wj"),
        r#"
use crate::base::Base

pub struct Wrapper {
    pub base: Base,
}

impl Wrapper {
    pub fn update(self, dt: f32) {
        self.base.update(dt)
    }
}
"#,
    )
    .unwrap();

    // Module file
    fs::write(
        pkg_dir.join("mod.wj"),
        r#"
pub mod base
pub mod wrapper
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let result = build_project(&pkg_dir, &output_dir, CompilationTarget::Rust, true);

    assert!(
        result.is_ok(),
        "Module build should succeed. Error: {}",
        result.err().unwrap()
    );

    // Check generated code has f32, not f64
    let base_rs = fs::read_to_string(output_dir.join("base.rs")).unwrap();
    assert!(
        base_rs.contains("0.0_f32") || base_rs.contains("0.0f32"),
        "Literal should be f32 in base.rs. Got:\n{}",
        base_rs
    );
    assert!(
        !base_rs.contains("0.0_f64") && !base_rs.contains("0.0f64"),
        "Should not have f64 literals in base.rs"
    );

    let wrapper_rs = fs::read_to_string(output_dir.join("wrapper.rs")).unwrap();
    assert!(
        wrapper_rs.contains("f32") || wrapper_rs.contains("update"),
        "Wrapper should have f32 types or update method"
    );
}

/// Test with mod.wj as entry point - matches `wj build mod.wj -o build --library`
/// When path is mod.wj, find_wj_files gets all files in parent directory
#[test]
fn test_mod_wj_entry_point_no_conflict() {
    let temp_dir = TempDir::new().unwrap();
    let pkg_dir = temp_dir.path().join("pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    fs::write(
        pkg_dir.join("base.wj"),
        r#"
pub struct Base {
    pub value: f32,
}

impl Base {
    pub fn update(self, dt: f32) {
        if self.value > 0.0 {
            self.value = self.value - dt
        }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        pkg_dir.join("wrapper.wj"),
        r#"
use crate::base::Base

pub struct Wrapper {
    pub base: Base,
}

impl Wrapper {
    pub fn update(self, dt: f32) {
        self.base.update(dt)
    }
}
"#,
    )
    .unwrap();

    fs::write(
        pkg_dir.join("mod.wj"),
        r#"
pub mod base
pub mod wrapper
"#,
    )
    .unwrap();

    let output_dir = temp_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    // Build with mod.wj as entry (like: wj build mod.wj -o build --library)
    let mod_wj_path = pkg_dir.join("mod.wj");
    let result = build_project_ext(
        &mod_wj_path,
        &output_dir,
        CompilationTarget::Rust,
        true,
        true, // library
        &[],
    );

    assert!(
        result.is_ok(),
        "Module build from mod.wj entry should succeed. Error: {}",
        result.err().unwrap()
    );

    let base_rs = fs::read_to_string(output_dir.join("base.rs")).unwrap();
    assert!(
        base_rs.contains("0.0_f32") || base_rs.contains("0.0f32"),
        "Literal should be f32 when built from mod.wj. Got:\n{}",
        base_rs
    );
}
