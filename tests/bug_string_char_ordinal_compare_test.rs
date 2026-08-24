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

//! FAILING REPRO — comparing single-char `string` slices with `<` / `>` must compile.
//!
//! Ecosystem `wj-todo-cli` used `ch < "0" || ch > "9"` after `strings.substring`.
//! Codegen kept owned `String` and compared to `&str` → E0277 PartialOrd.
//! Workaround: explicit digit equality (`ch == "0" || …`).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn string_char_ordinal_compare_must_compile() {
    let generated = test_utils::compile_single(
        r#"
use std::strings

pub fn is_digit_char(text: string) -> bool {
    if strings.len(text) != 1 {
        return false
    }
    let ch = strings.substring(text, 0, 1)
    ch >= "0" && ch <= "9"
}
"#,
    );

    assert!(
        generated.contains("as_str()")
            || generated.contains(".as_str()")
            || generated.contains("&*")
            || generated.contains("PartialOrd"),
        "substring char compare should emit comparable &str (or char) forms:\n{generated}"
    );
}
