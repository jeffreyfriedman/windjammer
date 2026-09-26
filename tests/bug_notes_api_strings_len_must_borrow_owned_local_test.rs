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

//! `strings.len(owned)` must borrow so the local can be used again.
//!
//! Ecosystem `wj-notes-api` `etag_matches` / `apply_cors` / `split_cors_origins`:
//! E0382 after `strings::len(want)` moves `String`, then `want == tag`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn strings_len_must_borrow_owned_local_for_later_use() {
    let source = r#"
use std::strings

pub fn etag_matches(if_none_match: string, tag: string) -> bool {
    let want = strings.trim(if_none_match)
    if strings.len(want) == 0 {
        return false
    }
    want == tag
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    eprintln!("generated:\n{generated}");
    assert!(
        generated.contains("strings::len(&want)")
            || generated.contains("strings::len(& want)"),
        "strings.len must borrow owned want, not move:\n{generated}"
    );
    assert!(
        ok,
        "etag_matches must cargo-check after strings.len borrow:\n{generated}"
    );
}
