//! Integration tests: parallel declaration stub registry preserves multipass behavior.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;

/// Regression: declaration stub collection must register module-qualified ownership.
#[test]
fn test_declaration_stub_registry_module_qualified_autoborrow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        src.join("draw.wj"),
        r#"
extern fn renderer_draw_text(text: String, x: f32, y: f32)

pub fn draw_text(text: string, x: f32, y: f32) {
    renderer_draw_text(text, x, y)
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("game.wj"),
        r#"
use crate::draw

pub fn render() {
    let label = format!("Score: {}", 42)
    draw::draw_text(label, 10.0, 20.0)
}
"#,
    )
    .unwrap();

    let out = dir.path().join("out");
    let output = test_utils::run_wj_command([
        "build",
        "--no-cargo",
        "--library",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "multipass build failed:\n{}",
        stderr
    );

    let code = fs::read_to_string(out.join("game.rs")).expect("game.rs");
    assert!(
        code.contains("&label"),
        "expected auto-borrow for draw::draw_text:\n{}",
        code
    );
}

/// Nested module struct fields must appear in the struct field registry.
#[test]
fn test_declaration_stub_registry_nested_struct_fields() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        src.join("config.wj"),
        r#"
pub mod tuning {
    pub struct Params {
        gain: f32,
    }
}

pub fn default_gain(p: tuning::Params) -> f32 {
    p.gain
}
"#,
    )
    .unwrap();

    let out = dir.path().join("out");
    let output = test_utils::run_wj_command([
        "build",
        "--no-cargo",
        "--library",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);

    assert!(
        output.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
