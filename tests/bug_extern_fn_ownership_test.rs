use std::fs;
/// TDD test: extern fn calls should NOT apply ownership inference
///
/// Bug: When calling extern fn with owned String argument, codegen adds `&`
/// because ownership inference marks the parameter as Borrowed. But extern
/// functions have explicit types - inference shouldn't override them.
///
/// Example: `ffi::render_text(text.to_string())` generates
/// `ffi::render_text(&text.to_string())` - wrong! Should be `ffi::render_text(text.to_string())`
///
/// Root Cause: Ownership inference runs on extern function parameters and
/// codegen applies `&`/`&mut` wrapping without checking if the function is extern.
///
/// Fix: Skip ownership inference for extern function call arguments.
use std::process::Command;

fn transpile_wj(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let _output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    fs::read_to_string(&rust_file).expect("Failed to read generated Rust file")
}

#[test]
fn test_extern_fn_call_no_ownership_inference() {
    let source = r#"
extern fn render_text(text: String, x: f32, y: f32)

pub fn draw_text(label: &str, x: f32, y: f32) {
    render_text(label.to_string(), x, y)
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // The call should NOT have & wrapping the argument
    // Should be: render_text(label.to_string(), x, y)
    // NOT: render_text(&label.to_string(), x, y)
    assert!(
        !generated.contains("&label.to_string()"),
        "extern fn call should NOT add & to String argument. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("render_text(label.to_string()"),
        "extern fn call should pass owned String directly. Got:\n{}",
        generated
    );
}

#[test]
fn test_extern_fn_call_no_mut_inference() {
    let source = r#"
extern fn update_pos(x: f32, y: f32)

pub fn move_to(pos_x: f32, pos_y: f32) {
    update_pos(pos_x, pos_y)
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // Should NOT add &mut to arguments for extern functions
    assert!(
        !generated.contains("&mut pos_x"),
        "extern fn call should NOT add &mut to args. Got:\n{}",
        generated
    );
}
