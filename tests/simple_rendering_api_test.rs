/// TDD Test: Simple Rendering API (without full game engine dependency)
///
/// This test verifies that the rendering API functions can be generated
/// correctly from Windjammer code without depending on the full game engine.
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_rendering_functions_compile() {
    let code = r#"
    // Minimal FFI declarations for rendering
    extern fn renderer_clear(r: f32, g: f32, b: f32, a: f32) {}
    extern fn renderer_draw_rect(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32) {}
    extern fn renderer_draw_circle(x: f32, y: f32, radius: f32, r: f32, g: f32, b: f32, a: f32) {}
    
    pub fn clear_color(r: f32, g: f32, b: f32, a: f32) {
        unsafe { renderer_clear(r, g, b, a) }
    }
    
    pub fn draw_rect(x: f32, y: f32, width: f32, height: f32, r: f32, g: f32, b: f32, a: f32) {
        unsafe { renderer_draw_rect(x, y, width, height, r, g, b, a) }
    }
    
    pub fn draw_circle(x: f32, y: f32, radius: f32, r: f32, g: f32, b: f32, a: f32) {
        unsafe { renderer_draw_circle(x, y, radius, r, g, b, a) }
    }
    
    fn main() {
        clear_color(0.0, 0.0, 0.0, 1.0)
        draw_rect(10.0, 10.0, 100.0, 50.0, 1.0, 0.0, 0.0, 1.0)
        draw_circle(200.0, 200.0, 25.0, 0.0, 1.0, 0.0, 1.0)
    }
    "#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    std::fs::write(&input_file, code).expect("Failed to write source file");

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    // Compile the Windjammer code
    let output = Command::new(&wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.join("build").to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run compiler");

    assert!(
        output.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that the generated Rust code contains our functions
    let generated_file = test_dir.join("build/test.rs");
    let generated =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated file");

    assert!(
        generated.contains("pub fn clear_color"),
        "Generated code should contain clear_color function"
    );
    assert!(
        generated.contains("pub fn draw_rect"),
        "Generated code should contain draw_rect function"
    );
    assert!(
        generated.contains("pub fn draw_circle"),
        "Generated code should contain draw_circle function"
    );
}
