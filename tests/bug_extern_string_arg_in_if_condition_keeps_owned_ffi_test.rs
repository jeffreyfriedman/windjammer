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

/// An extern string argument in an `if` condition, with the parameter used again afterwards, converts an owned
/// clone (`string_to_ffi(path.clone())`). The if-condition forward-ref pass used to rewrite that clone into
/// `string_to_ffi(&path)`, which rustc rejects (expected `String`, found `&String`).
#[test]
fn test_extern_string_arg_in_if_condition_is_not_borrowed() {
    let source = r#"
extern fn wj_file_exists(path: string) -> bool
extern fn wj_file_read(path: string) -> string

pub fn load(path: string) -> string {
    if !wj_file_exists(path) {
        println("missing {}", path)
        return ""
    }
    wj_file_read(path)
}
"#;

    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("string_to_ffi(&"),
        "string_to_ffi takes an owned String, never a borrow. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("wj_file_exists(windjammer_runtime::ffi::string_to_ffi(path.clone()))"),
        "the condition's extern call converts an owned clone. Got:\n{}",
        generated
    );
}
