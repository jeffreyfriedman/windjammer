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

// Test: Trait method signatures should use EXACT types from source (no inference)
// Bug: Trait methods with default implementations were getting ownership inference
// Expected: `delta: f32` in source → `delta: f32` in generated (NOT `delta: &f32`)

use std::process::Command;

fn compile_and_check(code: &str) -> (bool, String) {
    // Use unique temp directory per test to avoid parallel test conflicts
    // Use thread ID to ensure uniqueness even within same test binary
    let thread_id = format!("{:?}", std::thread::current().id());
    let _tmp = tempfile::tempdir().unwrap();
    let temp_dir = _tmp.path().join(format!("trait_test_{}", thread_id));

    let test_file = temp_dir.join("test.wj");
    let output_dir = temp_dir.join("output");

    std::fs::create_dir_all(&output_dir).ok();
    std::fs::write(&test_file, code).unwrap();

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

    let generated_file = output_dir.join("test.rs");
    let generated = std::fs::read_to_string(&generated_file).unwrap_or_default();

    // Clean up

    (output.status.success(), generated)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_no_inference_f32() {
    let code = r#"
pub trait GameLoop {
    fn update(delta: f32) {
        // Default implementation
    }
}
"#;

    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");

    // Check that trait signature uses f32, not &f32
    assert!(
        generated.contains("fn update(&self, delta: f32)"),
        "Empty default impl does not mutate — trait uses &self. Generated:\n{}",
        generated
    );

    // Ensure it's NOT generating &f32
    assert!(
        !generated.contains("delta: &f32"),
        "Trait method should NOT have '&f32'. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_no_inference_struct() {
    let code = r#"
pub struct Input { pub key: int }
pub struct RenderContext { pub width: int }

pub trait GameLoop {
    fn update(input: Input) {
        // Default implementation
    }
    
    fn render(ctx: RenderContext) {
        // Default implementation
    }
}
"#;

    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");

    // Check that trait signatures use owned types, not references
    assert!(
        generated.contains("fn update(&self, input: Input)"),
        "Empty default impl does not mutate — trait uses &self. Generated:\n{}",
        generated
    );
    assert!(generated.contains("fn render(&self, ctx: RenderContext)"), 
        "Trait method should have 'ctx: RenderContext' (owned), not '&RenderContext'. Generated:\n{}", generated);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_can_use_references() {
    let code = r#"
pub struct Input { pub key: int }

pub trait GameLoop {
    fn update(input: Input);
}

pub struct MyGame {}

impl GameLoop for MyGame {
    fn update(input: Input) {
        // Implementation - can use references if needed
        println(input.key)
    }
}
"#;

    let (success, generated) = compile_and_check(code);
    assert!(success, "Should compile successfully");

    // Trait signature should be owned
    assert!(
        generated.contains("fn update(&self, input: Input);"),
        "Read-only impl keeps trait at &self. Generated:\n{}",
        generated
    );

    // Implementation matches trait signature
    // (The implementation will also have Input because it must match the trait)
}
