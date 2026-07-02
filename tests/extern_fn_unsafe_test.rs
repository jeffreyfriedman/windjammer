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

/// TDD Test: Extern function calls should be automatically wrapped in unsafe blocks
///
/// THE WINDJAMMER WAY: The compiler should handle unsafe details so users don't have to.
/// When calling extern functions, the compiler should automatically add unsafe blocks.
#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_calls_wrapped_in_unsafe() {
    let code = r#"
    extern fn unsafe_function(x: i32) -> i32 {}
    
    pub fn safe_wrapper(x: i32) -> i32 {
        unsafe_function(x)
    }
    
    fn main() {
        let result = safe_wrapper(42)
    }
    "#;

    match test_utils::compile_single_result(code) {
        Ok(generated) => {
            // Check that the extern call is wrapped in unsafe
            assert!(
                generated.contains("unsafe") && generated.contains("unsafe_function"),
                "Extern function calls should be automatically wrapped in unsafe blocks.\nGenerated code:\n{}",
                generated
            );
        }
        Err(err) => {
            panic!("Compilation failed: {}", err);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_zero_arg_extern_fn_wrapped_in_unsafe() {
    let code = r#"
    extern fn deferred_geometry_execute() -> bool {}
    extern fn gi_execute() -> bool {}
    extern fn clear_lights() {}
    
    pub fn run_geometry() -> bool {
        deferred_geometry_execute()
    }
    
    pub fn run_gi() -> bool {
        gi_execute()
    }
    
    pub fn do_clear() {
        clear_lights()
    }
    
    fn main() {
        let a = run_geometry()
        let b = run_gi()
        do_clear()
    }
    "#;

    match test_utils::compile_single_result(code) {
        Ok(generated) => {
            // All extern function calls should be wrapped in unsafe blocks,
            // regardless of whether they have arguments or not
            assert!(
                generated.contains("unsafe") && generated.contains("deferred_geometry_execute"),
                "Zero-arg extern fn 'deferred_geometry_execute' should be wrapped in unsafe.\nGenerated code:\n{}",
                generated
            );
            assert!(
                generated.contains("unsafe") && generated.contains("gi_execute"),
                "Zero-arg extern fn 'gi_execute' should be wrapped in unsafe.\nGenerated code:\n{}",
                generated
            );
            assert!(
                generated.contains("unsafe") && generated.contains("clear_lights"),
                "Zero-arg extern fn 'clear_lights' should be wrapped in unsafe.\nGenerated code:\n{}",
                generated
            );

            // Verify with rustc
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let rust_file = temp_dir.path().join("test.rs");
            std::fs::write(&rust_file, &generated).expect("Failed to write Rust file");
            let rustc_output = Command::new("rustc")
                .arg("--crate-type")
                .arg("lib")
                .arg("--emit")
                .arg("metadata")
                .arg(&rust_file)
                .arg("--out-dir")
                .arg(temp_dir.path())
                .output()
                .expect("Failed to run rustc");
            assert!(
                rustc_output.status.success(),
                "Generated Rust for zero-arg extern calls should compile.\nrustc stderr:\n{}\nGenerated code:\n{}",
                String::from_utf8_lossy(&rustc_output.stderr),
                generated
            );
        }
        Err(err) => {
            panic!("Compilation failed: {}", err);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_cross_module_zero_arg_extern_fn_unsafe() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "api.wj",
        r#"
extern fn deferred_geometry_execute() -> bool {}
extern fn render_deferred_frame() -> bool {}
extern fn gi_execute() -> bool {}
extern fn ssr_execute() -> bool {}
extern fn deferred_lighting_clear_lights() {}
"#,
    );
    test.add_file(
        "gpu_safe.wj",
        r#"
use crate::api

pub fn deferred_geometry_execute() -> bool {
    api::deferred_geometry_execute()
}

pub fn render_deferred_frame() -> bool {
    api::render_deferred_frame()
}

pub fn gi_execute() -> bool {
    api::gi_execute()
}

pub fn ssr_execute() -> bool {
    api::ssr_execute()
}

pub fn deferred_lighting_clear_lights() {
    api::deferred_lighting_clear_lights()
}
"#,
    );

    let results = test.compile().expect("compile");
    let gpu_safe = results
        .get("gpu_safe.rs")
        .expect("gpu_safe.rs should be generated");

    // Every api:: call in gpu_safe should have unsafe wrapper
    for func in &[
        "deferred_geometry_execute",
        "render_deferred_frame",
        "gi_execute",
        "ssr_execute",
        "deferred_lighting_clear_lights",
    ] {
        let has_unsafe_call = gpu_safe.lines().any(|line| {
            line.contains(&format!("api::{}", func)) && line.contains("unsafe")
        });
        assert!(
            has_unsafe_call,
            "Cross-module zero-arg extern call 'api::{}' should be wrapped in unsafe.\nGenerated gpu_safe.rs:\n{}",
            func, gpu_safe
        );
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_rendering_api_with_extern_calls() {
    let code = r#"
    extern fn renderer_clear(r: f32, g: f32, b: f32, a: f32) {}
    
    pub fn clear_color(r: f32, g: f32, b: f32, a: f32) {
        renderer_clear(r, g, b, a)
    }
    
    fn main() {
        clear_color(0.0, 0.0, 0.0, 1.0)
    }
    "#;

    match test_utils::compile_single_result(code) {
        Ok(generated) => {
            // The generated code should compile with rustc
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let rust_file = temp_dir.path().join("test.rs");
            std::fs::write(&rust_file, &generated).expect("Failed to write Rust file");

            // Compile to object file only (don't link) since extern fn has no implementation
            let rustc_output = Command::new("rustc")
                .arg("--crate-type")
                .arg("lib")
                .arg("--emit")
                .arg("metadata") // Just check syntax, don't link
                .arg(&rust_file)
                .arg("--out-dir")
                .arg(temp_dir.path())
                .output()
                .expect("Failed to run rustc");

            assert!(
                rustc_output.status.success(),
                "Generated Rust code should compile with rustc.\nrustc stderr:\n{}\nGenerated code:\n{}",
                String::from_utf8_lossy(&rustc_output.stderr),
                generated
            );
        }
        Err(err) => {
            panic!("Compilation failed: {}", err);
        }
    }
}
