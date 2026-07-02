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

//! Float literal type inference for Vec containers.
//!
//! Bug: When a Vec is populated with bare float literals (0.5, 0.3) and later
//! passed to a function expecting Vec<f32>, the compiler should infer Vec<f32>
//! for the local variable. Instead it defaults to Vec<f64>, causing a type
//! mismatch between the container element type and the literal suffixes.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn test_vec_float_infers_f32_from_function_context() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rendering/buffers.wj",
        r#"
pub fn process_pixels(data: Vec<f32>, width: u32) -> f32 {
    let mut sum = 0.0
    let mut i = 0
    while i < data.len() {
        sum = sum + data[i]
        i = i + 1
    }
    sum
}

pub fn create_test_buffer() -> f32 {
    let mut pixels = Vec::new()
    pixels.push(0.5)
    pixels.push(0.3)
    pixels.push(0.7)
    pixels.push(1.0)
    process_pixels(pixels, 2)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("rendering/buffers.rs").expect("buffers.rs");

    // The Vec should be inferred as Vec<f32> (not Vec<f64>) based on
    // how it's used: passed to process_pixels which takes Vec<f32>.
    assert!(
        !rs.contains("Vec<f64>"),
        "Vec should be inferred as f32, not f64, from function context. Got:\n{rs}"
    );
}

#[test]
fn test_vec_float_with_explicit_f32_struct_field() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "data/container.wj",
        r#"
pub struct PixelBuffer {
    pub data: Vec<f32>,
    pub width: u32,
}

pub fn make_buffer() -> PixelBuffer {
    let mut pixels = Vec::new()
    pixels.push(0.5)
    pixels.push(0.3)
    PixelBuffer { data: pixels, width: 2 }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("data/container.rs").expect("container.rs");

    assert!(
        !rs.contains("Vec<f64>"),
        "Vec assigned to Vec<f32> struct field should be inferred as f32. Got:\n{rs}"
    );
}
