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

//! Multipass: `strings.len(owned)` must borrow so the local can be used again.
//!
//! Single-file already emits `strings::len(&want)`. Hexagonal `wj-notes-api`
//! `domain/api.wj` `etag_matches` still emits `strings::len(want)` then
//! `want == tag` → E0382. Do not reshape the app.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn strings_len_must_borrow_owned_local_for_later_use() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod domain
"#,
    );
    test.add_file(
        "domain/mod.wj",
        r#"
pub mod api
"#,
    );
    test.add_file(
        "domain/api.wj",
        r#"
use std::strings

pub fn etag_matches(if_none_match: string, tag: string) -> bool {
    let want = strings.trim(if_none_match)
    if strings.len(want) == 0 {
        return false
    }
    want == tag
}
"#,
    );

    let map = test.compile().expect("multipass compile");
    let api_rs = map
        .get("domain/api.rs")
        .or_else(|| map.get("api.rs"))
        .expect("api.rs");
    eprintln!("domain/api.rs:\n{api_rs}");
    assert!(
        api_rs.contains("strings::len(&want)") || api_rs.contains("strings::len(& want)"),
        "multipass strings.len must borrow owned want, not move:\n{api_rs}"
    );

    test.cargo_check().unwrap_or_else(|e| {
        panic!("etag_matches must cargo-check after strings.len borrow:\n{e}\n{api_rs}")
    });
}
