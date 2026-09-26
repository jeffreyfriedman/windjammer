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

//! Phase-2: comparison-only `decode_store(text: string)` demotes to `&str`.
//! Match-arm `Ok(body)` must borrow into that formal (`decode_store(&body)`), not
//! keep a dual-oracle owned API just so the payload can move.

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
        codec_rs.contains("fn decode_store(text: &str)"),
        "comparison-only string formal demotes to &str. emitted:\n{codec_rs}"
    );
    assert!(
        codec_rs.contains("decode_store(&body")
            || codec_rs.contains("decode_store(body.as_str()")
            || codec_rs.contains("decode_store(&*body"),
        "owned match binding must borrow into demoted &str formal. emitted:\n{codec_rs}"
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
        adapter.contains("decode_store(&body")
            || adapter.contains("decode_store(body.as_str()")
            || adapter.contains("decode_store(&*body"),
        "hexagonal adapter must borrow owned match binding into demoted &str. emitted:\n{adapter}"
    );
}
