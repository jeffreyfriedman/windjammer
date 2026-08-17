#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! FAILING REPRO — `map.get("KEY")` inside `match Ok(map)` must stay `&str`, not `.to_string()`.
//!
//! Match arms that bind `HashMap<string, _>` enable blanket string-literal ownership
//! coercion; HashMap::get still expects borrowed keys. Ecosystem `wj-dotenv` hits this
//! in `match load(...) { Ok(map) => map.get("APP") ... }`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn hashmap_get_string_literal_in_result_match_arm_stays_borrowed() {
    let generated = test_utils::compile_single(
        r#"
use std::collections::HashMap

pub fn load_app() -> Result<HashMap<string, string>, string> {
    let mut map = HashMap::new()
    map.insert("APP", "ecosystem")
    Ok(map)
}

pub fn has_app() -> bool {
    match load_app() {
        Ok(map) => map.get("APP").is_some(),
        Err(_) => false,
    }
}
"#,
    );

    assert!(
        !generated.contains("get(\"APP\".to_string())")
            && !generated.contains("get(\"APP\".to_owned())"),
        "HashMap::get key literal must not be owned in match arm:\n{generated}"
    );
    assert!(
        generated.contains("get(\"APP\")"),
        "expected borrowed string literal key:\n{generated}"
    );
}
