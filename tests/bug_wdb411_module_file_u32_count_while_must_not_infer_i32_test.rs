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
    feature = "codegen_tests",
))]

//! WDB-411: `while i < count` must stay u32 when `count` is u32, even if the
//! body uses f32 math and `as i64` indexing (frame_analysis leftover).
//!
//! P3.348 isolate omitted luminance / `as i64` and stayed u32. Product
//! `is_black` leftover rustc:
//!   `while i < count` — expected `i32`, found `u32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct FramePixels {
    pub data: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

impl FramePixels {
    pub fn pixel_count(self) -> u32 {
        self.width * self.height
    }
}

pub fn luminance(r: f32, g: f32, b: f32) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub fn is_black(frame: FramePixels) -> bool {
    let threshold = 0.001
    let count = frame.pixel_count()
    let mut total_lum = 0.0
    let mut i = 0
    while i < count {
        let idx = (i * 4) as i64
        let lum = luminance(frame.data[idx], frame.data[idx + 1], frame.data[idx + 2])
        total_lum = total_lum + lum
        i = i + 1
    }
    let avg = total_lum / count as f32
    avg < threshold
}

/// Product leftover: early `count == 0` return poisons `i` to i32 (average_brightness).
pub fn average_brightness(frame: FramePixels) -> f32 {
    let count = frame.pixel_count()
    if count == 0 { return 0.0 }
    let mut total = 0.0
    let mut i = 0
    while i < count {
        let idx = (i * 4) as i64
        total = total + luminance(frame.data[idx], frame.data[idx + 1], frame.data[idx + 2])
        i = i + 1
    }
    total / count as f32
}
"#;

#[test]
fn wdb411_module_file_u32_count_while_must_not_infer_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-411 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-411 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("let mut i: i32")
            && !rs.contains("let mut i = 0_i32")
            && !rs.contains("0_i32"),
        "WDB-411 RED: u32 count loop inferred i32:\n{rs}"
    );
    assert!(
        rs.contains("while i < count"),
        "expected while i < count:\n{rs}"
    );
    test.cargo_check()
        .expect("WDB-411 u32 count while must cargo-check");
}
