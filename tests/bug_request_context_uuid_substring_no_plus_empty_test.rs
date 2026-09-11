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

//! GREEN regression — LedgerKit `request_context` UUID / Bearer substring without
//! `+ ""`. Tip must emit usize indices (not `String as usize`) and cargo-check.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str = include_str!("fixtures/library_multipass/request_context_uuid_substring.wj");

fn assert_request_context_emit(rs: &str) {
    assert!(
        rs.contains("substring"),
        "must emit substring; got:\n{rs}"
    );
    assert!(
        !rs.contains("String as usize"),
        "must not cast String as usize; got:\n{rs}"
    );
    // Reject empty-concat temps in the UUID / bearer path (fixture has none).
    assert!(
        !rs.contains(r#"format!("{}{}", hex, "")"#)
            && !rs.contains(r#"format!("{}{}", owned, "")"#),
        "fixture must not emit empty-concat temps; got:\n{rs}"
    );
}

#[test]
fn request_context_uuid_substring_must_cargo_check_without_plus_empty() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert_request_context_emit(&rs);
    assert!(
        ok,
        "RED: request_context UUID/Bearer without + \"\" must cargo-check. Generated:\n{rs}"
    );
}

#[test]
fn hexagonal_request_context_uuid_substring_must_cargo_check_without_plus_empty() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod ids\n");
    project.add_file("domain/ids.wj", SOURCE);
    project.add_file("adapters/mod.wj", "pub mod inbound\n");
    project.add_file(
        "adapters/inbound.wj",
        r#"
use super::super::domain::ids::{bearer_token_from_authorization, generate_request_id}

pub fn request_id_for(method: string, path: string, auth: string) -> string {
    let token = match bearer_token_from_authorization(auth) {
        Some(t) => t,
        None => "",
    }
    generate_request_id(method, path, token)
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal request_context compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("ids.rs"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "missing ids.rs; keys: {:?}",
                map.keys().collect::<Vec<_>>()
            )
        });
    assert_request_context_emit(map.get(&key).expect("ids.rs"));
}
