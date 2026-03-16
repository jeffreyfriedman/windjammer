//! TDD: Function parameter float type propagation
//!
//! Bug: Function parameters with explicit f32/f64 type aren't propagating to float
//! literals used with them. Example: `fn update(self, dt: f32) { dt * 1000.0 }`
//! generates 1000.0 as f64 instead of f32.
//!
//! Root Cause: Function parameters are registered in var_types but the constraint
//! propagation may not be working correctly for impl methods.

use tempfile::TempDir;
use windjammer::{build_project, CompilationTarget};

fn compile_to_rust(source: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    let output_dir = test_dir.join("build");

    std::fs::create_dir_all(&output_dir).expect("Failed to create output dir");
    std::fs::write(&input_file, source).expect("Failed to write source file");

    build_project(&input_file, &output_dir, CompilationTarget::Rust, true)
        .map_err(|e| format!("Windjammer compilation failed: {}", e))?;

    let output_file = output_dir.join("test.rs");
    let rust_code = std::fs::read_to_string(&output_file)
        .map_err(|e| format!("Failed to read generated file: {}", e))?;

    Ok(rust_code)
}

#[test]
fn test_f32_param_propagates_to_literals() {
    // Matches breach-protocol settings/performance.wj update() method exactly
    let source = r#"
pub struct PerformanceStats {
    pub frame_count: u64,
    pub total_time: f32,
    pub fps: f32,
    pub frame_time_ms: f32,
}

impl PerformanceStats {
    pub fn update(self, dt: f32) {
        self.frame_time_ms = dt * 1000.0
        self.fps = if dt > 0.0 { 1.0 / dt } else { 0.0 }
    }
}
"#;

    let rust = compile_to_rust(source).unwrap();

    // Literals should be inferred as f32 from dt: f32
    assert!(
        rust.contains("1000.0_f32") || rust.contains("1000.0f32"),
        "1000.0 should be f32 when used with dt: f32, got:\n{}",
        rust
    );
    assert!(
        rust.contains("1.0_f32") || rust.contains("1.0f32"),
        "1.0 should be f32 when used with dt: f32, got:\n{}",
        rust
    );
    assert!(
        rust.contains("0.0_f32") || rust.contains("0.0f32"),
        "0.0 in else branch should be f32, got:\n{}",
        rust
    );
}

#[test]
fn test_f64_param_propagates_to_literals() {
    let source = r#"
pub fn compute(timestamp: f64) {
    let delta = timestamp - 1000.0
}
"#;

    let rust = compile_to_rust(source).unwrap();
    assert!(
        rust.contains("1000.0_f64") || rust.contains("1000.0f64"),
        "1000.0 should be f64 when used with timestamp: f64, got:\n{}",
        rust
    );
}

/// Multi-file build: mimics breach-protocol structure with settings/performance.wj
#[test]
fn test_f32_param_in_multi_file_build() {
    use std::fs;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let settings_dir = test_dir.join("settings");
    fs::create_dir_all(&settings_dir).expect("Failed to create settings dir");

    // Create mod.wj (like breach-protocol)
    let mod_content = r#"
pub mod performance
"#;
    fs::write(settings_dir.join("mod.wj"), mod_content).expect("Failed to write mod.wj");

    // Create performance.wj (exact copy of breach-protocol's update method)
    let perf_content = r#"
pub struct PerformanceStats {
    pub frame_count: u64,
    pub total_time: f32,
    pub fps: f32,
    pub frame_time_ms: f32,
}

impl PerformanceStats {
    pub fn update(self, dt: f32) {
        self.frame_time_ms = dt * 1000.0
        self.fps = if dt > 0.0 { 1.0 / dt } else { 0.0 }
    }
}
"#;
    fs::write(settings_dir.join("performance.wj"), perf_content).expect("Failed to write performance.wj");

    let output_dir = test_dir.join("build");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    // Build from directory (like wj build breach-protocol)
    let result = build_project(&settings_dir, &output_dir, CompilationTarget::Rust, true);

    let rust = match result {
        Ok(()) => {
            let perf_rs = output_dir.join("performance.rs");
            if perf_rs.exists() {
                fs::read_to_string(&perf_rs).unwrap_or_default()
            } else {
                // Try settings/performance.rs
                fs::read_to_string(output_dir.join("settings").join("performance.rs")).unwrap_or_default()
            }
        }
        Err(e) => panic!("Build failed: {}", e),
    };

    assert!(
        rust.contains("1000.0_f32") || rust.contains("1000.0f32"),
        "Multi-file: 1000.0 should be f32 when used with dt: f32, got:\n{}",
        rust
    );
    assert!(
        rust.contains("1.0_f32") || rust.contains("1.0f32"),
        "Multi-file: 1.0 should be f32, got:\n{}",
        rust
    );
    assert!(
        rust.contains("0.0_f32") || rust.contains("0.0f32"),
        "Multi-file: 0.0 in else branch should be f32, got:\n{}",
        rust
    );
}
