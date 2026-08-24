//! FAILING REPRO — `std::yaml` must exist for config files (PyYAML-scale ubiquity).
//!
//! Ecosystem `wj-yaml` is a pure-WJ subset workaround. Language adoption needs
//! `use std::yaml` with parse → structured value (or JSON text) and stringify.
//!
//! Gate: transpile must not emit `compile_error!("missing boundary signature…")`
//! and generated code must `cargo check` against `windjammer-runtime`.

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
fn std_yaml_parse_to_json_codegen_resolves() {
    let source = r#"
use std::yaml

pub fn load(text: string) -> Result<string, string> {
    yaml.to_json(text)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["yaml::to_json"]);
}

#[test]
fn std_yaml_parse_value_codegen_resolves() {
    let source = r#"
use std::yaml

pub fn has_name(text: string) -> bool {
    match yaml.parse(text) {
        Ok(_) => true,
        Err(_) => false,
    }
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["yaml::parse"]);
}
