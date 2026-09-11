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

//! Multipass `json.get_index(root, i)` in `while` loop must auto-borrow owned `Value`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const CODEC: &str = include_str!("fixtures/library_multipass/codec_json_get_index_loop.wj");

#[test]
fn json_get_index_owned_value_multipass_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod codec
pub use codec::count_array_items
"#,
    );
    project.add_file("codec.wj", CODEC);

    let map = project.compile().expect("codec multipass compile should succeed");
    let codec_rs = map.get("codec.rs").expect("codec.rs");
    assert!(
        codec_rs.contains("get_index(&root") || codec_rs.contains("get_index(& root"),
        "json.get_index must auto-borrow owned Value; emitted:\n{codec_rs}"
    );

    project
        .cargo_check()
        .expect("multipass json.get_index loop on owned Value must cargo-check");
}
