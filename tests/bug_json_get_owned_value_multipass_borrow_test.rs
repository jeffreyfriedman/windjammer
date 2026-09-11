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

//! Multipass `json.get` / nested `json.is_array` / `json.len` on owned `Value` after
//! `json.parse` must auto-borrow (`&Value`). `wj-json-util` class.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const CODEC: &str = include_str!("fixtures/library_multipass/codec_json_field_get.wj");

#[test]
fn json_get_owned_value_multipass_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod codec
pub use codec::field_count
"#,
    );
    project.add_file("codec.wj", CODEC);

    let map = project.compile().expect("codec multipass compile should succeed");
    let codec_rs = map.get("codec.rs").expect("codec.rs");
    assert!(
        codec_rs.contains("get(&root") || codec_rs.contains("get(& root"),
        "json.get must auto-borrow owned Value root; emitted:\n{codec_rs}"
    );
    assert!(
        codec_rs.contains("is_array(&v)") || codec_rs.contains("is_array(& v)"),
        "json.is_array must auto-borrow owned Value; emitted:\n{codec_rs}"
    );
    assert!(
        codec_rs.contains("len(&v)") || codec_rs.contains("len(& v)"),
        "json.len must auto-borrow owned Value; emitted:\n{codec_rs}"
    );
    assert!(
        codec_rs.contains("as i64"),
        "json.len usize return must cast to i64; emitted:\n{codec_rs}"
    );

    project
        .cargo_check()
        .expect("multipass json.get/is_array/len on owned Value must cargo-check");
}
