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

//! FAILING REPRO — `Ok(body) => decode_store(body)` must move owned `string` into owned formal,
//! not emit `decode_store(&body)` (`wj-todo-cli` import branch).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const DECODE: &str = include_str!("fixtures/library_multipass/domain_decode_store.wj");
const IMPORT: &str = include_str!("fixtures/library_multipass/adapters_import_snapshot.wj");
const LOAD_SNAPSHOT: &str = include_str!("fixtures/library_multipass/match_binding_owned_decode.wj");

#[test]
fn owned_match_binding_cross_fn_owned_string_formal_must_move() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod codec
pub use codec::load_snapshot
"#,
    );
    project.add_file("codec.wj", LOAD_SNAPSHOT);

    let map = project
        .compile()
        .expect("load_snapshot multipass compile should succeed");
    let codec_rs = map.get("codec.rs").expect("codec.rs");
    assert!(
        !codec_rs.contains("decode_store(&body") && !codec_rs.contains("decode_store( &body"),
        "RED: must move owned match binding into owned formal; emitted:\n{codec_rs}"
    );
}

#[test]
fn owned_match_binding_hexagonal_cross_module_must_move() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod decode_store\n");
    project.add_file("domain/decode_store.wj", DECODE);
    project.add_file("adapters/mod.wj", "pub mod import_snapshot\n");
    project.add_file("adapters/import_snapshot.wj", IMPORT);

    let map = project
        .compile()
        .expect("hexagonal import_snapshot compile should succeed");
    let adapter = map
        .get("adapters/import_snapshot.rs")
        .expect("adapters/import_snapshot.rs");
    assert!(
        !adapter.contains("decode_store(&body") && !adapter.contains("decode_store( &body"),
        "RED: hexagonal adapter must move owned match binding; emitted:\n{adapter}"
    );
}
