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

//! FAILING REPRO — owned `HashMap` helper with `match map.get` must not inject
//! mid-match `std::thread::spawn(move || drop(map))`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn hashmap_owned_get_helper_must_not_inject_mid_match_defer_drop() {
    let generated = test_utils::compile_single(
        r#"
use std::collections::HashMap

pub fn int_from_map(map: HashMap<string, string>, key: string, fallback: int) -> int {
    match map.get(key) {
        Some(v) => {
            let _ = v
            fallback
        },
        None => fallback,
    }
}
"#,
    );

    assert!(
        !generated.contains("thread::spawn"),
        "owned HashMap .get helper must not emit defer-drop spawn:\n{generated}"
    );
    assert!(
        generated.contains("match map.get") || generated.contains("match map.get("),
        "expected map.get match retained:\n{generated}"
    );
}
