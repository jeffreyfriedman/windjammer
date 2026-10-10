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

//! Regression: module file owned string key must not receive ref lsqb push adj.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// `.len()`-only helpers demote to `&str`. Force owned String via struct store.
const SRC: &str = r#"
pub struct Adj {
    pub key: string,
    pub neighbor: i64,
}

pub fn push_adj(key: string, neighbor: i64) -> Adj {
    Adj {
        key: key,
        neighbor: neighbor,
    }
}

pub fn add_edge(edge_kind: string, src: i64, dst: i64) -> Adj {
    let out_key = edge_kind + "_out"
    let in_key = edge_kind + "_in"
    let a = push_adj(out_key, dst)
    let _b = push_adj(in_key, src)
    a
}
"#;

#[test]
fn module_file_owned_string_key_must_not_receive_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn push_adj(key: String")
        || rs.contains("fn push_adj(mut key: String");
    assert!(
        owned,
        "expected owned String key formal on push_adj:\n{rs}"
    );
    let bad = rs.contains("&out_key") || rs.contains("&in_key") || rs.contains("push_adj(&");
    assert!(
        !bad,
        "RED: owned push_adj received &out_key/&in_key:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
