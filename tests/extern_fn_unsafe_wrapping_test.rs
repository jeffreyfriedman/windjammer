#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_extern_fn_call_wrapped_in_unsafe() {
    let code = r#"
extern fn renderer_clear(handle: u32, r: f32, g: f32, b: f32, a: f32)

fn clear_screen() {
    renderer_clear(0, 0.0, 0.0, 0.0, 1.0)
}
"#;
    let rust = test_utils::compile_single(code);
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
    let rust = test_utils::compile_single(code);
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
    let rust = test_utils::compile_single(code);
    println!("{}", rust);
    assert!(
        rust.contains("unsafe"),
        "Extern fn call with return value should be wrapped in unsafe block.\nGenerated:\n{}",
        rust
    );
}
