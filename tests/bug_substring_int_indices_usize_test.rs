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

//! FAILING REPRO — `strings.substring(s, i, i+1)` with `int` loop indices must compile.
//!
//! Ecosystem `wj-validate` hit E0308: `expected usize, found i64` when codegen emits
//! `strings::substring(&s, j, j + 1)` without casting Windjammer `int` to `usize`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn substring_int_indices_must_cast_to_usize() {
    let generated = test_utils::compile_single(
        r#"
use std::strings

pub fn first_at(text: string) -> int {
    let mut i = 0
    while i < strings.len(text) {
        if strings.substring(text, i, i + 1) == "@" {
            return i
        }
        i = i + 1
    }
    -1
}
"#,
    );

    let has_cast = generated.contains("as usize")
        || generated.contains("try_into()")
        || generated.contains("usize::try_from");
    assert!(
        has_cast,
        "substring int indices must be cast to usize in generated Rust:\n{generated}"
    );
}
