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

//! FAILING REPRO — read-only helper params must be borrowed, not owned.
//!
//! Ecosystem `wj-validate`:
//! ```ignore
//! let trimmed = strings.trim(value)
//! if !looks_like_email(trimmed) { ... }
//! Ok(trimmed)  // E0382: moved into looks_like_email
//! ```
//! `looks_like_email` only reads `text`; ownership inference must emit `&String` / `&str`,
//! not `String`, so the caller can still return `trimmed`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn readonly_helper_param_must_borrow_not_own() {
    let generated = test_utils::compile_single(
        r#"
use std::strings

pub fn require_email(field: string, value: string) -> Result<string, string> {
    let trimmed = strings.trim(value)
    if strings.len(trimmed) == 0 {
        return Err("required")
    }
    if !looks_like_email(trimmed) {
        return Err("bad email")
    }
    Ok(trimmed)
}

fn looks_like_email(text: string) -> bool {
    strings.contains(text, "@")
}
"#,
    );

    assert!(
        !generated.contains("looks_like_email(trimmed)")
            || generated.contains("looks_like_email(&trimmed)")
            || generated.contains("looks_like_email(trimmed.as_str())"),
        "read-only helper must borrow trimmed so Ok(trimmed) compiles:\n{generated}"
    );
    // Stronger: generated Rust must compile (no move of trimmed).
    assert!(
        generated.contains("&trimmed")
            || generated.contains("trimmed.as_str()")
            || generated.contains("&*trimmed"),
        "expected borrow of trimmed at looks_like_email call site:\n{generated}"
    );
}
