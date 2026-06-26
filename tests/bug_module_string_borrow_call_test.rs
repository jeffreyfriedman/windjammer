#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

/// Module-level `string` formals are owned `String`; callers pass owned locals by value.
#[test]
fn test_internal_module_call_borrows_string_arg() {
    let source = r#"
pub fn load_shader(path: string) -> u32 {
    0
}

pub fn build() -> u32 {
    let shader_path = "foo.wjsl".to_string()
    load_shader(shader_path)
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("load_shader(shader_path)")
            && !generated.contains("load_shader(&shader_path)"),
        "owned String local should pass by value to owned string param. Got:\n{}",
        generated
    );
}
