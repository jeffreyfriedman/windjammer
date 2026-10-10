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

//! Regression: json merge overlay loop reuse must cargo check.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const CODEC: &str = include_str!("fixtures/library_multipass/codec_json_merge_overlay_loop.wj");

#[test]
fn json_merge_overlay_loop_reuse_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod codec
pub use codec::count_overlay_fields
"#,
    );
    project.add_file("codec.wj", CODEC);

    let map = project.compile().expect("merge overlay loop multipass compile");
    let codec_rs = map.get("codec.rs").expect("codec.rs");
    eprintln!("P3.669 codec.rs take_field / loop sites:\n{codec_rs}");

    // Accept either demoted `&Value` formal or clone-on-reuse at the call site.
    let take_sig_borrowed = codec_rs.contains("fn take_field(value: &Value")
        || codec_rs.contains("fn take_field(value: & Value");
    let call_clones_or_borrows = codec_rs.contains("take_field(overlay.clone()")
        || codec_rs.contains("take_field(&overlay")
        || codec_rs.contains("take_field(& overlay");
    assert!(
        take_sig_borrowed || call_clones_or_borrows,
        "P3.669 RED: owned overlay reused in loop into owned take_field must borrow \
         formal or clone/borrow at call site; emitted:\n{codec_rs}"
    );

    project
        .cargo_check()
        .expect("P3.669 multipass merge overlay loop reuse must cargo-check");
}
