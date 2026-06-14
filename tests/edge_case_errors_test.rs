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

//! TDD: E0053 / E0599 when inherent and trait impl share a method name (dogfooding voxel_gpu_renderer).
//!
//! Rust requires trait impl signatures and bodies to come from the trait impl block, not the
//! inherent method with the same name.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn assert_rustc_lib_ok(rust: &str) {
    let temp_dir = TempDir::new().unwrap();
    let rs_file = temp_dir.path().join("test.rs");
    fs::write(&rs_file, rust).unwrap();
    let rmeta = temp_dir.path().join("verify.rmeta");
    let rustc_output = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(rmeta.to_str().unwrap())
        .arg(rs_file.to_str().unwrap())
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

    let output = test_utils::compile_single(source);
    let port_impl = output
        .find("impl Port for Gpu")
        .expect("trait impl present");
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
