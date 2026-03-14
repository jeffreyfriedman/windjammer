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
        "wj_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Compilation failed:\nSTDERR:\n{}\nSTDOUT:\n{}",
            stderr, stdout
        );
    }

    let rust_file = out_dir.join("test.rs");
    let content = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    let _ = fs::remove_dir_all(&test_dir);

    content
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
    // Should be: render_text(string_to_ffi(label.to_string()), x, y)
    // NOT: render_text(&label.to_string(), x, y)
    assert!(
        !generated.contains("&label.to_string()"),
        "extern fn call should NOT add & to String argument. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("string_to_ffi(label.to_string())"),
        "extern fn call should pass String via string_to_ffi. Got:\n{}",
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

/// TDD test: extern fn string args should NOT have double .to_string()
///
/// Bug: When user writes render_text(label.to_string(), x, y), codegen produced
/// string_to_ffi(label.to_string().to_string()) - double conversion!
///
/// Root Cause: Expression generation produces "label.to_string()", then extern
/// wrapping added another .to_string() before string_to_ffi().
///
/// Fix: Strip trailing .to_string() from arg_str before adding string_to_ffi wrapper.
#[test]
fn test_extern_fn_no_double_to_string() {
    let source = r#"
extern fn render_text(text: String, x: f32, y: f32)

pub fn draw_text(label: &str, x: f32, y: f32) {
    render_text(label.to_string(), x, y)
}
"#;

    let generated = transpile_wj(source);
    println!("Generated:\n{}", generated);

    // Should NOT have double .to_string().to_string()
    assert!(
        !generated.contains(".to_string().to_string()"),
        "extern fn call should NOT have double .to_string(). Got:\n{}",
        generated
    );
    // Should have single .to_string() inside string_to_ffi
    assert!(
        generated.contains("string_to_ffi(label.to_string())"),
        "extern fn call should have single string_to_ffi(label.to_string()). Got:\n{}",
        generated
    );
}
