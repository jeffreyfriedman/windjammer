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

//! `std::config` — consolidated config process (toml/yaml format is impl detail).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_config_to_json_toml_must_wire() {
    let source = r#"
use std::config

pub fn load(text: string) -> Result<string, string> {
    config.to_json(text, "toml")
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["config::to_json"]);
}

#[test]
fn std_config_to_json_yaml_must_wire() {
    let source = r#"
use std::config

pub fn load(text: string) -> Result<string, string> {
    config.to_json(text, "yaml")
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["config::to_json"]);
}

#[test]
fn std_config_parse_flat_must_wire() {
    let source = r#"
use std::config
use std::collections::HashMap

pub fn load(text: string) -> Result<HashMap<string, string>, string> {
    config.parse_flat(text, "toml")
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["config::parse_flat"]);
}

#[test]
fn std_config_resolve_must_wire() {
    let source = r#"
use std::config
use std::collections::HashMap

pub fn build(
    defaults: HashMap<string, string>,
    file: HashMap<string, string>,
    env_map: HashMap<string, string>,
) -> HashMap<string, string> {
    config.resolve(defaults, file, env_map)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["config::resolve"]);
}
