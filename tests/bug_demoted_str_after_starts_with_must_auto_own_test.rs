//! After read-only `strings.starts_with` / `ends_with` on an owned `string`
//! param, a later call to an owned `string` formal must auto-own (`.to_string()`),
//! not pass the demoted `&str`.
//!
//! Ecosystem `wj-toml` inline-table expand:
//! ```
//! fn expand_entries(key: string, raw: string) -> Vec<(string, string)> {
//!     if strings.starts_with(raw, "{") {
//!         if strings.ends_with(raw, "}") {
//!             return expand_inline_table(key, raw)
//!         }
//!     }
//!     let mut out = Vec::new()
//!     out.push((key, normalize_value(raw)))  // owned formal
//!     out
//! }
//! ```
//! Tip/0.50 demotes `raw` to `&str` for the whole fn → E0308 at `normalize_value`.

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
fn demoted_str_after_starts_with_must_auto_own_into_owned_formal() {
    let source = r#"
use std::strings

fn normalize_value(raw: string) -> string {
    raw
}

pub fn expand_entries(key: string, raw: string) -> string {
    if strings.len(raw) >= 2 {
        if strings.starts_with(raw, "{") {
            if strings.ends_with(raw, "}") {
                return key
            }
        }
    }
    normalize_value(raw)
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "demoted &str after starts_with must auto-own into owned formal, got:\n{generated}"
    );
    // GREEN if either: (1) keep `raw: String` owned through expand_entries, or
    // (2) demote to `&str` but auto-own at the owned normalize_value call.
    let keeps_owned = generated.contains("raw: String")
        && generated.contains("normalize_value(raw)")
        && !generated.contains("fn expand_entries(key: String, raw: &str)");
    let auto_owns = generated.contains("normalize_value(raw.to_string())")
        || generated.contains("normalize_value(raw.clone()")
        || generated.contains("normalize_value(raw.to_owned()");
    let bare_demoted_pass = (generated.contains("raw: &str")
        || generated.contains("fn expand_entries(key: String, raw: &str)"))
        && generated.contains("normalize_value(raw)")
        && !auto_owns;
    assert!(
        (keeps_owned || auto_owns) && !bare_demoted_pass,
        "expected owned raw or auto-own into normalize_value, got:\n{generated}"
    );
}
