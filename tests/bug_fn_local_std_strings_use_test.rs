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

//! FAILING REPRO — function-local `use std::strings` must bind Windjammer strings.
//!
//! `wj test` emits an inner `use std::strings` as Rust `use std::strings`
//! (E0432: no `strings` in the root). The same import at file scope resolves.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn fn_local_std_strings_use_must_cargo_check() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "len.wj",
        r#"
pub fn len_of(text: string) -> int {
    use std::strings
    strings.len(text)
}
"#,
    );

    test.cargo_check()
        .expect("function-local use std::strings must resolve like a file-scope import");
}
