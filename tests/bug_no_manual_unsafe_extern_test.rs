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

/// Windjammer source must not use `unsafe` for extern fn calls — compiler handles FFI wrapping.
#[test]
fn test_wj_source_extern_call_has_no_unsafe_keyword() {
    let source = r#"
extern fn playtest_agent_mode_enabled() -> u32

pub fn agent_mode_enabled() -> bool {
    playtest_agent_mode_enabled() != 0
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    // Windjammer parser should reject or not emit user `unsafe` — only compiler wraps extern.
    assert!(
        generated.contains("unsafe { playtest_agent_mode_enabled"),
        "compiler should auto-wrap extern call in unsafe. Got:\n{}",
        generated
    );
}
