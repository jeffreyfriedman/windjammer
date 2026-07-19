#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// `use std::map::Map` must map to std::collections::HashMap, not windjammer_runtime::map.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_map_import_uses_hashmap_alias() {
    let source = r#"
use std::map::Map

pub fn empty_map() -> Map<string, i32> {
    Map::new()
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("windjammer_runtime::map"),
        "Map import must not use nonexistent windjammer_runtime::map.\nGenerated:\n{generated}"
    );
    assert!(
        generated.contains("std::collections::HashMap") || generated.contains("HashMap"),
        "Map should lower to HashMap.\nGenerated:\n{generated}"
    );
    test_utils::verify_rust_compiles(&generated).expect("generated Rust should compile");
}
