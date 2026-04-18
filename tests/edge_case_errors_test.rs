//! TDD: E0053 / E0599 when inherent and trait impl share a method name (dogfooding voxel_gpu_renderer).
//!
//! Rust requires trait impl signatures and bodies to come from the trait impl block, not the
//! inherent method with the same name.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_and_get_rust(source: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = PathBuf::from(format!(
        "/tmp/edge_case_errors_{}_{}",
        std::process::id(),
        counter
    ));

    std::fs::create_dir_all(&test_dir).unwrap();

    let source_file = test_dir.join("test.wj");
    std::fs::write(&source_file, source).unwrap();

    windjammer::build_project(
        &source_file,
        &test_dir,
        windjammer::CompilationTarget::Rust,
        true,
    )
    .expect("Failed to compile Windjammer code");

    let rust_file = test_dir.join("test.rs");
    std::fs::read_to_string(&rust_file).expect("Failed to read generated Rust file")
}

fn assert_rustc_lib_ok(rust: &str) {
    let temp_dir = TempDir::new().unwrap();
    let rs_file = temp_dir.path().join("test.rs");
    fs::write(&rs_file, rust).unwrap();
    let out_lib = temp_dir.path().join("lib.rlib");
    let rustc_output = Command::new("rustc")
        .args([
            rs_file.to_str().unwrap(),
            "--crate-type=lib",
            "-o",
            out_lib.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");
    assert!(
        rustc_output.status.success(),
        "Generated Rust should compile. stderr:\n{}",
        String::from_utf8_lossy(&rustc_output.stderr)
    );
}

#[test]
fn test_trait_impl_not_confused_with_inherent_same_method_name() {
    let source = r#"
pub struct LightingData {
    pub v: f32
}

pub struct LightingConfig {
    pub v: f32
}

pub trait Port {
    fn set_lighting(data: LightingData)
}

pub struct Gpu {
    pub cfg: LightingConfig
}

impl Gpu {
    pub fn set_lighting(self, config: LightingConfig) {
        self.cfg = config
    }
}

impl Port for Gpu {
    fn set_lighting(data: LightingData) {
        self.cfg = LightingConfig { v: data.v }
    }
}
"#;

    let output = compile_and_get_rust(source);
    let port_impl = output.find("impl Port for Gpu").expect("trait impl present");
    let tail = &output[port_impl..];

    assert!(
        tail.contains("fn set_lighting(&mut self, data: LightingData)"),
        "Trait impl must use trait param type LightingData (not inherent LightingConfig); got:\n{tail}"
    );
    assert!(
        !tail.contains("fn set_lighting(&mut self, config: LightingConfig)"),
        "Trait impl must not reuse inherent signature; got:\n{tail}"
    );
    assert!(
        tail.contains("LightingConfig { v: data.v }"),
        "Trait impl body should map LightingData into cfg; got:\n{tail}"
    );

    assert_rustc_lib_ok(&output);
}
