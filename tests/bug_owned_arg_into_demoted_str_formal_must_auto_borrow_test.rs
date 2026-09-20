//! Owned `string` args into demoted `&str` formals must auto-borrow (E0308 otherwise).
//!
//! Ecosystem: `wj-notes-api` → `wj-validate::require_nonempty(field, value)` and
//! `wj-url` → `wj-querystring` thin-wrap attempts fail with `expected &str, found String`.

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
fn owned_arg_into_demoted_str_formal_must_cargo_check() {
    let source = r#"
use std::strings

fn require_nonempty(field: string, value: string) -> Result<string, string> {
    if strings.len(value) == 0 {
        return Err("${field} required")
    }
    Ok(value)
}

pub fn check_title(title: string) -> Result<string, string> {
    require_nonempty("title", title)
}
"#;
    // Force demotion pattern: helper takes string (may become &str) + call with owned literal path
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "owned args into string formals must cargo-check (auto-borrow if demoted), got:\n{generated}"
    );
}
