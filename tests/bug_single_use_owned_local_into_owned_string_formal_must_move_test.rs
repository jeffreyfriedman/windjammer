//! Single-use owned local into owned `string` formal must move, not `&local`.
//!
//! Ecosystem `wj-toml` tests under tip `wj`:
//! ```
//! let text = "items = []\n"
//! match get(text, "items") { ... }
//! ```
//! Tip emits `get(&text, …)` → E0308 expected `String`, found `&String`.

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
fn single_use_owned_local_into_owned_string_formal_must_move() {
    let source = r#"
pub fn get(text: string, key: string) -> Option<string> {
    Some(text)
}

pub fn sample() -> Option<string> {
    let text = "items = []"
    get(text, "items")
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "single-use owned local into owned string formal must compile, got:\n{generated}"
    );
    assert!(
        !generated.contains("get(&text"),
        "must move text into get, not borrow:\n{generated}"
    );
}
