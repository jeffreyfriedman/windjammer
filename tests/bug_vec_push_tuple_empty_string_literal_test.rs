//! Empty string literals inside `Vec::push((…, ""))` must own as `String`.
//!
//! Ecosystem `wj-querystring`: `pairs.push((decode_component(part), ""))`.
//! `Ok((text, ""))` is already gated green; bare push into `Vec<(string, string)>`
//! still leaves a bare `&str` (E0308).

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn empty_string_literal_in_vec_push_tuple_owns_string() {
    let source = r#"
pub fn push_flag(parts: Vec<(string, string)>, key: string) -> Vec<(string, string)> {
    let mut out = parts
    out.push((key, ""))
    out
}
"#;
    let generated = test_utils::compile_single(source);
    let owns = generated.contains("String::new()")
        || generated.contains("\"\".to_string()")
        || generated.contains("String::from(\"\")");
    assert!(
        owns,
        "Vec::push((key, \"\")) must own empty string, got:\n{generated}"
    );
    assert!(
        !generated.contains("out.push((key, \"\"))")
            && !generated.contains(".push((key, \"\"))"),
        "must not leave bare &str \"\" in Vec push tuple, got:\n{generated}"
    );
}
