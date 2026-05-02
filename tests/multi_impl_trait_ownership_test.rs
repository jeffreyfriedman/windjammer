//! TDD: Trait receiver ownership when multiple impls disagree (mut vs no-op).
//! Contract: if ANY impl needs &mut self, trait and all impls use &mut self.

#[path = "test_utils.rs"]
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
fn test_multi_impl_one_mutates_one_noop_trait_uses_mut_self() {
    let source = r#"
pub trait Counter {
    fn tick()
}

pub struct RealCounter {
    count: u32
}

impl Counter for RealCounter {
    fn tick() {
        self.count = self.count + 1_u32
    }
}

pub struct NoOpCounter {}

impl Counter for NoOpCounter {
    fn tick() {
    }
}
"#;

    let output = test_utils::compile_single(source);
    eprintln!("\n=== Generated Rust ===\n{output}\n");

    assert!(
        output.contains("pub trait Counter") && output.contains("fn tick(&mut self)"),
        "Trait should use fn tick(&mut self) when any impl mutates; got:\n{output}"
    );

    let real_impl = output.find("impl Counter for RealCounter").unwrap();
    let noop_impl = output.find("impl Counter for NoOpCounter").unwrap();
    assert!(
        real_impl < noop_impl,
        "Expected RealCounter impl before NoOpCounter in output"
    );
    let between = &output[real_impl..noop_impl];
    assert!(
        between.contains("fn tick(&mut self)"),
        "RealCounter impl should use &mut self; got:\n{between}"
    );
    assert!(
        output[noop_impl..].contains("fn tick(&mut self)"),
        "NoOpCounter impl should match trait &mut self; got:\n{}",
        &output[noop_impl..]
    );

    assert_rustc_lib_ok(&output);
}

#[test]
fn test_multi_impl_both_read_only_trait_uses_ref_self() {
    let source = r#"
pub trait Reader {
    fn get_value() -> u32
}

pub struct Source1 {
    value: u32
}

impl Reader for Source1 {
    fn get_value() -> u32 {
        self.value
    }
}

pub struct Source2 {
    other_value: u32
}

impl Reader for Source2 {
    fn get_value() -> u32 {
        self.other_value
    }
}
"#;

    let output = test_utils::compile_single(source);
    eprintln!("\n=== Generated Rust (Reader) ===\n{output}\n");

    assert!(
        output.contains("fn get_value(&self) -> u32"),
        "Read-only impls should yield &self; got:\n{output}"
    );
    assert_rustc_lib_ok(&output);
}

#[test]
fn test_multi_impl_indirect_mut_via_inherent_method_on_same_type() {
    // Regression: widen pass used only the trait impl block for `current_impl_functions`, so
    // `self.init_gpu()` did not see the inherent `init_gpu` body → trait stayed `&self`.
    let source = r#"
pub trait RenderPort {
    fn initialize()
}

pub struct MockRenderer {}

impl RenderPort for MockRenderer {
    fn initialize() {
    }
}

pub struct VoxelGPURenderer {
    ready: bool
}

impl VoxelGPURenderer {
    fn init_gpu(self) {
        self.ready = true
    }
}

impl RenderPort for VoxelGPURenderer {
    fn initialize() {
        self.init_gpu()
    }
}
"#;

    let output = test_utils::compile_single(source);
    eprintln!("\n=== Generated Rust (RenderPort indirect) ===\n{output}\n");

    assert!(
        output.contains("pub trait RenderPort") && output.contains("fn initialize(&mut self)"),
        "Trait should be &mut self when an impl mutates via inherent helper; got:\n{output}"
    );
    assert!(
        output.matches("fn initialize(&mut self)").count() >= 2,
        "Mock and Voxel impls should both use &mut self to match trait; got:\n{output}"
    );
    assert_rustc_lib_ok(&output);
}

#[test]
fn test_multi_impl_both_mutate_trait_uses_mut_self() {
    let source = r#"
pub trait Updater {
    fn update()
}

pub struct Thing1 {
    x: i32
}

impl Updater for Thing1 {
    fn update() {
        self.x = self.x + 1
    }
}

pub struct Thing2 {
    y: i32
}

impl Updater for Thing2 {
    fn update() {
        self.y = self.y + 1
    }
}
"#;

    let output = test_utils::compile_single(source);
    eprintln!("\n=== Generated Rust (Updater) ===\n{output}\n");

    assert!(
        output.contains("fn update(&mut self)"),
        "Both mutating impls should yield &mut self everywhere; got:\n{output}"
    );
    assert_rustc_lib_ok(&output);
}
