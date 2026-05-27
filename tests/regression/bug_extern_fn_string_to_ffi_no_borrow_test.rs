#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Extern fn string args use `string_to_ffi(...)` by value — never `&string_to_ffi(...)` (E0308).
#[test]
fn test_extern_call_string_to_ffi_not_borrowed() {
    let source = r#"
extern fn file_exists(path: string) -> u32

pub fn path_exists(path: string) -> bool {
    file_exists(path) != 0
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("string_to_ffi("),
        "extern string arg should use string_to_ffi. Got:\n{}",
        generated
    );
    assert!(
        !generated.contains("&string_to_ffi(")
            && !generated.contains("&windjammer_runtime::ffi::string_to_ffi("),
        "must not borrow string_to_ffi result (FfiString is owned). Got:\n{}",
        generated
    );
}
