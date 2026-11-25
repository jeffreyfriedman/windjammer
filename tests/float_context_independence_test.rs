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

//! A float literal takes its width from the operand it meets (a declaration or an inferred type), never from the
//! enclosing function's return type or from the target of a cast it sits under.
//!
//! * `(x * 3.0) as f32` with `x: f64`: the product is f64; only the cast result is f32.
//! * `if x > 0.5 { .. }` with `x: f64` inside a function returning f32.
//! * `assert_eq(GAIN, 0.5)` where `GAIN: f32` is declared in another module of the library under test.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

fn assert_checks_and_keeps_f64(source: &str, literal: &str) {
    let (output, ok) = test_utils::compile_single_check(source);
    assert!(ok, "the program must pass numeric inference, got:\n{output}");
    assert!(
        output.contains(&format!("{literal}_f64")),
        "`{literal}` must stay f64 (its operand is f64), got:\n{output}"
    );
    assert!(
        !output.contains(&format!("{literal}_f32")),
        "`{literal}` must not adopt f32 from the cast target or the return type, got:\n{output}"
    );
}

#[test]
fn cast_operand_literal_does_not_adopt_the_cast_target() {
    assert_checks_and_keeps_f64(
        r#"
pub fn narrow(x: f64) -> f32 {
    (x * 3.25) as f32
}
"#,
        "3.25",
    );
}

#[test]
fn let_bound_cast_operand_literal_does_not_adopt_the_return_type() {
    assert_checks_and_keeps_f64(
        r#"
pub fn narrow(x: f64) -> f32 {
    let y = x * 3.25
    y as f32
}
"#,
        "3.25",
    );
}

#[test]
fn untyped_let_literal_does_not_adopt_the_return_type() {
    assert_checks_and_keeps_f64(
        r#"
pub fn narrow(x: f64) -> f32 {
    let k = 0.125
    (x * k) as f32
}
"#,
        "0.125",
    );
}

#[test]
fn comparison_literal_does_not_adopt_the_return_type() {
    assert_checks_and_keeps_f64(
        r#"
pub fn pick(x: f64) -> f32 {
    if x > 0.625 { 1.0 } else { 0.0 }
}
"#,
        "0.625",
    );
}

#[test]
fn call_argument_arithmetic_does_not_adopt_the_return_type() {
    assert_checks_and_keeps_f64(
        r#"
pub fn wide(v: f64) -> f64 { v }

pub fn narrow(x: f64) -> f32 {
    wide(x * 3.25) as f32
}
"#,
        "3.25",
    );
}

#[test]
fn return_position_literal_still_takes_the_return_type() {
    let output = test_utils::compile_single(
        r#"
pub fn half() -> f32 {
    let k = 0.5
    k
}
"#,
    );
    assert!(output.contains("0.5_f32"), "a returned literal is f32, got:\n{output}");
}

#[test]
fn assert_eq_literal_takes_a_local_const_width() {
    let output = test_utils::compile_single(
        r#"
const GAIN: f32 = 0.5

fn main() {
    assert_eq(GAIN, 0.5)
    assert_ne(GAIN, 0.75)
}
"#,
    );
    assert!(output.contains("0.5_f32") && output.contains("0.75_f32"), "got:\n{output}");
    assert!(!output.contains("_f64"), "no f64 literal beside an f32 constant, got:\n{output}");
}

/// `wj test` compiles each test file on its own against the library's metadata: a constant the library declares
/// reaches the test file through that metadata, so the literal beside it takes the constant's width.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn assert_eq_literal_takes_an_imported_const_width_under_wj_test() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("wj.toml"),
        "[package]\nname = \"floatconst\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[lib]\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.wj"),
        "pub const GAIN: f32 = 0.5\npub const RATE: u32 = 44100\npub fn gain() -> f32 { GAIN }\n",
    )
    .unwrap();
    fs::write(
        root.join("tests/consts_test.wj"),
        r#"use floatconst::{GAIN, RATE, gain}

@test
fn imported_consts_keep_their_declared_widths() {
    assert_eq(GAIN, 0.5)
    assert_ne(GAIN, 0.75)
    assert(GAIN < 1.0)
    assert_eq(GAIN * 2.0, 1.0)
    assert_eq(gain(), 0.5)
    assert_eq(RATE, 44100)
}
"#,
    )
    .unwrap();

    let output = Command::new(test_utils::wj_binary())
        .arg("test")
        .current_dir(root)
        .output()
        .expect("run wj test");
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "a literal beside an imported f32 constant must be f32 under `wj test`:\n{combined}"
    );
    assert!(combined.contains("1 passed"), "expected the test to run and pass:\n{combined}");
}
