//! User-defined `join(string, string)` must not inherit `strings::join` call-site borrowing.
//!
//! Ecosystem `wj-url`: `pub fn join(base, relative)` + `use std::strings` caused test call
//! sites to emit `join(&base, &relative)` (E0308). Renaming to `join_url` avoided the clash;
//! resolution must be signature/module-aware, not bare-name.

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
fn user_join_two_strings_moves_owned_locals() {
    let source = r#"
use std::strings

pub fn join(base: string, relative: string) -> Result<string, string> {
    if strings.contains(relative, "://") {
        return Ok(relative)
    }
    let mut parts = Vec::new()
    parts.push("x")
    let _ = strings.join(parts, "/")
    Ok("${base}/${relative}")
}

pub fn resolve() -> string {
    let base = "https://example.com/a/b"
    let relative = "c"
    match join(base, relative) {
        Ok(out) => out,
        Err(e) => e,
    }
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("join(base, relative)")
            || generated.contains("join(base.clone(), relative.clone())"),
        "user join(base, relative) must move owned strings, got:\n{generated}"
    );
    assert!(
        !generated.contains("join(&base, &relative)"),
        "must not borrow as if strings::join, got:\n{generated}"
    );
}
