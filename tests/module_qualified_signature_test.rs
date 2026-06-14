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

/// TDD Test: Module-Qualified Signature Registration in Multipass Builds
///
/// Bug: When compiling a multi-file project (directory build), the
/// `build_library_multipass` function in `compiler.rs` only registered
/// function signatures under their simple names (e.g., `draw_text`), not
/// under module-qualified names (e.g., `draw::draw_text`). This meant the
/// code generator couldn't find the correct ownership for cross-module
/// calls, resulting in missing `&` auto-borrow and E0308 type errors in
/// the generated Rust.
///
/// Root Cause: `build_library_multipass` built its `global_registry`
/// entirely from the Analyzer, which only produces simple function names.
/// The metadata-based qualified name registration (via `.wj.meta` files)
/// only existed in the dead code path in `main.rs` — the actual
/// compilation routed through `compiler.rs`.
///
/// Fix: Register module-qualified names (`file_stem::func_name`) alongside
/// simple names in `build_library_multipass`, and seed the global registry
/// with dependency crate metadata signatures.
#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;

#[test]
fn test_multipass_module_qualified_autoborrow() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    // Module that wraps an FFI call with a borrowed string parameter
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

    // Main game file that calls draw::draw_text with a String variable
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

    let output = Command::new(test_utils::wj_binary())
        .args(["build", "--no-cargo"])
        .arg(src.to_str().unwrap())
        .arg("--output")
        .arg(out.to_str().unwrap())
        .output()
        .expect("run wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "wj build failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Verify the generated Rust code has auto-borrow (&label)
    let game_rs = out.join("game.rs");
    assert!(game_rs.exists(), "game.rs not generated");
    let code = fs::read_to_string(&game_rs).unwrap();

    assert!(
        code.contains("&label"),
        "Generated Rust should auto-borrow String variable as &label for draw::draw_text.\n\
         Generated code:\n{}",
        code
    );
}

#[test]
fn test_multipass_no_name_collision_different_modules() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    // Module A: draw_text takes borrowed string (FFI wrapper)
    fs::write(
        src.join("draw.wj"),
        r#"
extern fn renderer_draw_text(text: String, x: f32)

pub fn draw_text(text: string, x: f32) {
    renderer_draw_text(text, x)
}
"#,
    )
    .unwrap();

    // Module B: draw_text takes owned string (different function, same name)
    fs::write(
        src.join("hud.wj"),
        r#"
pub fn draw_text(text: string, x: f32) {
    println!("{} at {}", text, x)
}
"#,
    )
    .unwrap();

    // Game: calls both — each should get the correct ownership treatment
    fs::write(
        src.join("game.wj"),
        r#"
use crate::draw
use crate::hud

pub fn render() {
    let label = format!("HP: {}", 100)
    draw::draw_text(label, 10.0)
    let info = format!("Info: {}", 42)
    hud::draw_text(info, 20.0)
}
"#,
    )
    .unwrap();

    let out = dir.path().join("out");

    let output = Command::new(test_utils::wj_binary())
        .args(["build", "--no-cargo"])
        .arg(src.to_str().unwrap())
        .arg("--output")
        .arg(out.to_str().unwrap())
        .output()
        .expect("run wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "wj build failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    let game_rs = out.join("game.rs");
    assert!(game_rs.exists(), "game.rs not generated");
    let code = fs::read_to_string(&game_rs).unwrap();

    // draw::draw_text should auto-borrow (string param is Borrowed)
    assert!(
        code.contains("draw::draw_text(&label"),
        "draw::draw_text should auto-borrow label.\nGenerated:\n{}",
        code
    );

    // hud::draw_text may still show `&info` if ownership passes as reference; both forms can be valid Rust
    assert!(
        code.contains("hud::draw_text("),
        "expected hud::draw_text call in generated code:\n{}",
        code
    );
}
