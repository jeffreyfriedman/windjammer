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
    feature = "codegen_tests",
))]

//!
//!   `pub fn new(name: impl Into<String>, …) { … name.into() … }`
//! Windjammer source is `name: string`. Prefer `name: String` + move (no `.into()`).
//! Rust leakage / non-idiomatic codegen.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct ConsoleCommand {
    pub name: string,
    pub description: string,
}

// stays concrete String; the compiler still emitted `impl Into<String>` for associated `new`.
impl ConsoleCommand {
    pub fn new(name: string, description: string) -> ConsoleCommand {
        ConsoleCommand {
            name: name,
            description: description,
        }
    }
}
"#;

#[test]
fn module_file_owned_string_must_not_emit_into() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("impl Into<")
        || rs.contains(".into()")
        || rs.contains("Into<String>");
    assert!(
        !bad,
        "RED: owned string emitted Into/into:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

