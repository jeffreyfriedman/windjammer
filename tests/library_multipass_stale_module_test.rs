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
))]

//! Stale hand-written `src/query.rs` must not become a root `pub mod query` when
//! `src/ecs/query.wj` is the real module (game-core E0583 regression).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::fs;

#[test]
fn test_stale_flat_rs_not_merged_when_subtree_has_wj_module() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "pub mod ecs;\n");
    test.add_file(
        "ecs/mod.wj",
        r#"
pub mod query

pub fn ecs_marker() -> i32 {
    1
}
"#,
    );
    test.add_file(
        "ecs/query.wj",
        r#"
pub fn query_marker() -> i32 {
    42
}
"#,
    );

    // Stale hand-written Rust at crate root (no matching query.wj at root).
    let stale = test.build_dir().parent().unwrap().join("src/query.rs");
    fs::write(
        &stale,
        "pub fn stale_root_query() -> i32 { 0 }\n",
    )
    .expect("write stale query.rs");

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let root_mod = fs::read_to_string(test.build_dir().join("mod.rs")).expect("root mod.rs");

    assert!(
        !root_mod.contains("pub mod query;"),
        "stale src/query.rs must not flatten ecs/query.wj to crate root. mod.rs:\n{root_mod}"
    );
    assert!(
        root_mod.contains("pub mod ecs;"),
        "ecs module must remain declared. mod.rs:\n{root_mod}"
    );
    assert!(
        map.contains_key("ecs/query.rs"),
        "ecs/query.rs should be generated"
    );
    let ecs_mod = fs::read_to_string(test.build_dir().join("ecs/mod.rs")).expect("ecs/mod.rs");
    assert!(
        ecs_mod.contains("pub mod query;"),
        "query must live under ecs/. ecs/mod.rs:\n{ecs_mod}"
    );
}
