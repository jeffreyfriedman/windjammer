//! WDB-170 class / `wj-toml`: demoted `&str` into owned tuple push must `.to_string()`.

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
fn eco_wj_toml_demoted_str_into_tuple_push_must_to_string() {
    let source = r#"
use std::strings

fn is_inline(raw: string) -> bool {
    strings.starts_with(raw, "{")
}

fn expand_entries(key: string, raw: string) -> Vec<(string, string)> {
    if is_inline(raw) {
        return Vec::new()
    }
    let mut out = Vec::new()
    out.push((key, "${raw}"))
    out
}

pub fn probe(key: string, raw: string) -> int {
    let v = expand_entries(key, raw)
    v.len() as int
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "wj-toml expand_entries push shape must compile, got:\n{generated}"
    );
    if generated.contains("fn expand_entries(key: &str") || generated.contains("key: &str,") {
        assert!(
            !generated.contains("key.clone()"),
            "demoted &str into owned tuple must not use .clone():\n{generated}"
        );
        assert!(
            generated.contains("key.to_string()"),
            "demoted &str into owned tuple must use .to_string():\n{generated}"
        );
    }
}
