// TDD Test: Trait implementations must match trait signatures EXACTLY
// Bug: Trait impls apply ownership inference when they should match trait signature
// Expected: impl fn update(delta: f32) to match trait fn update(delta: f32)

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_windjammer_and_check(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.wj");
    let output_dir = temp_dir.path().join("out");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    fs::write(&test_file, code).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            test_file.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj");

    let output_file = output_dir.join("test.rs");
    let generated = fs::read_to_string(&output_file).unwrap_or_default();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (output.status.success(), generated, stderr)
}

fn compile_rust(code: &str) -> (bool, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.rs");
    let output_file = temp_dir.path().join("libtest.rlib");

    fs::write(&test_file, code).unwrap();

    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            test_file.to_str().unwrap(),
            "-o",
            output_file.to_str().unwrap(),
            "--edition",
            "2021",
        ])
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stderr)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_matches_trait_signature_f32() {
    let code = r#"
pub trait GameLoop {
    fn update(&mut self, delta: f32) {}
}

pub struct MyGame {}

impl GameLoop for MyGame {
    fn update(&mut self, delta: f32) {
        let _used = delta + 1.0
    }
}
"#;

    let (success, generated, _stderr) = compile_windjammer_and_check(code);
    assert!(success, "Windjammer compilation should succeed");

    // Verify trait has f32 (not &f32)
    assert!(
        generated.contains("trait GameLoop"),
        "Should have trait definition"
    );
    assert!(
        generated.contains("fn update(&mut self, delta: f32)"),
        "Trait should have 'delta: f32'. Generated:\n{}",
        generated
    );

    // Verify impl ALSO has f32 (not &f32) to match trait
    assert!(
        generated.contains("impl GameLoop for MyGame"),
        "Should have impl"
    );

    // The critical check: impl must match trait signature
    let impl_section: String = generated
        .lines()
        .skip_while(|l| !l.contains("impl GameLoop for MyGame"))
        .take_while(|l| !l.starts_with("pub ") || l.contains("fn "))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        impl_section.contains("fn update(&mut self, delta: f32"),
        "Impl should have 'delta: f32' to match trait (not &f32). Impl section:\n{}",
        impl_section
    );

    // Verify Rust compilation succeeds (no E0053 error)
    let (rust_success, rust_stderr) = compile_rust(&generated);
    assert!(
        rust_success,
        "Generated Rust should compile without E0053 errors. Errors:\n{}",
        rust_stderr
    );
    assert!(
        !rust_stderr.contains("E0053"),
        "Should not have E0053 (incompatible type for trait). Stderr:\n{}",
        rust_stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_matches_trait_signature_struct() {
    let code = r#"
pub struct Input { pub key: int }
pub struct RenderContext { pub width: int }

pub trait GameLoop {
    fn update(&mut self, input: Input) {}
    fn render(&self, ctx: RenderContext) {}
}

pub struct MyGame {}

impl GameLoop for MyGame {
    fn update(&mut self, input: Input) {}
    fn render(&self, ctx: RenderContext) {}
}
"#;

    let (success, generated, _stderr) = compile_windjammer_and_check(code);
    assert!(success, "Windjammer compilation should succeed");

    // Verify trait signatures are owned (not &Input, &RenderContext)
    let trait_section: String = generated
        .lines()
        .skip_while(|l| !l.contains("trait GameLoop"))
        .take_while(|l| !l.starts_with("pub struct") && !l.starts_with("impl"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        trait_section.contains("fn update(&mut self, input: Input)"),
        "Trait should have 'input: Input'. Trait:\n{}",
        trait_section
    );
    assert!(
        trait_section.contains("fn render(&self, ctx: RenderContext)"),
        "Trait should have 'ctx: RenderContext'. Trait:\n{}",
        trait_section
    );

    // Verify impl signatures MATCH trait (not &Input, &RenderContext)
    let impl_section: String = generated
        .lines()
        .skip_while(|l| !l.contains("impl GameLoop for MyGame"))
        .take(20)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        impl_section.contains("fn update(&mut self, input: Input"),
        "Impl should match trait with 'input: Input'. Impl:\n{}",
        impl_section
    );
    assert!(
        impl_section.contains("fn render(&self, ctx: RenderContext"),
        "Impl should match trait with 'ctx: RenderContext'. Impl:\n{}",
        impl_section
    );

    // Verify Rust compilation succeeds
    let (rust_success, rust_stderr) = compile_rust(&generated);
    assert!(
        rust_success,
        "Generated Rust should compile. Errors:\n{}",
        rust_stderr
    );
    assert!(
        !rust_stderr.contains("E0053"),
        "Should not have E0053. Stderr:\n{}",
        rust_stderr
    );
}
