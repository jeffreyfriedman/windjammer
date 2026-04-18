use windjammer::compiler::build_project;
use windjammer::CompilationTarget;
use std::path::Path;

fn compile_to_rust(code: &str) -> String {
    let dir = std::env::temp_dir().join(format!(
        "wj_extern_unsafe_{}_{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("test.wj"), code).unwrap();
    let out = dir.join("build");
    build_project(Path::new(&dir.join("test.wj")), &out, CompilationTarget::Rust, false).unwrap();
    std::fs::read_to_string(out.join("test.rs")).unwrap()
}

#[test]
fn test_extern_fn_call_wrapped_in_unsafe() {
    let code = r#"
extern fn renderer_clear(handle: u32, r: f32, g: f32, b: f32, a: f32)

fn clear_screen() {
    renderer_clear(0, 0.0, 0.0, 0.0, 1.0)
}
"#;
    let rust = compile_to_rust(code);
    println!("{}", rust);
    assert!(
        rust.contains("unsafe"),
        "Direct extern fn call should be wrapped in unsafe block.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_extern_fn_call_via_module_path_wrapped_in_unsafe() {
    let code = r#"
mod ffi {
    extern fn renderer_clear(handle: u32, r: f32, g: f32, b: f32, a: f32)
}

fn clear_screen() {
    ffi::renderer_clear(0, 0.0, 0.0, 0.0, 1.0)
}
"#;
    let rust = compile_to_rust(code);
    println!("{}", rust);
    assert!(
        rust.contains("unsafe"),
        "Module-qualified extern fn call should be wrapped in unsafe block.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_extern_fn_with_return_value_wrapped_in_unsafe() {
    let code = r#"
extern fn gpu_render(vb: u32, ib: u32, count: u32) -> bool

fn render() -> bool {
    gpu_render(1, 2, 3)
}
"#;
    let rust = compile_to_rust(code);
    println!("{}", rust);
    assert!(
        rust.contains("unsafe"),
        "Extern fn call with return value should be wrapped in unsafe block.\nGenerated:\n{}",
        rust
    );
}
