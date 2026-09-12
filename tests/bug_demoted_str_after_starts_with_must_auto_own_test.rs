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

//! P3.251 (`wj-toml`): read-only `strings.starts_with` / `ends_with` must not leave a
//! demoted `&str` formal unable to pass into a later owned `string` formal.
//!
//! Product workaround: `normalize_value("${raw}")` force-own until this gate is green.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str = r#"
use std::strings

fn normalize_value(raw: string) -> string {
    raw
}

fn expand_entries(raw: string) -> string {
    if strings.starts_with(raw, "{") {
        if strings.ends_with(raw, "}") {
            return normalize_value(raw)
        }
    }
    normalize_value(raw)
}

pub fn cap(raw: string) -> string {
    expand_entries(raw)
}
"#;

#[test]
fn demoted_str_after_starts_with_must_auto_own_into_owned_formal() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    eprintln!("P3.251 generated:\n{rs}");

    // Either keep `raw` owned through expand_entries, or auto-own at normalize_value sites.
    let demoted = rs.contains("fn expand_entries(raw: &str)")
        || rs.contains("fn expand_entries(raw:&str)");
    let bare_pass_to_owned = rs.contains("normalize_value(raw)")
        && !rs.contains("normalize_value(raw.to_string())")
        && !rs.contains("normalize_value(raw.clone())");
    assert!(
        !(demoted && bare_pass_to_owned),
        "P3.251 RED: demoted &str after starts_with must auto-own into owned string formal. Got:\n{rs}"
    );
    assert!(
        ok,
        "P3.251 RED: fixture must cargo-check (demoted &str → owned formal). Generated:\n{rs}"
    );
}
