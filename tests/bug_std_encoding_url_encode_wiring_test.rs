//! `std::encoding.url_encode` / `url_decode` must `cargo check` for querystring work.
//!
//! Ecosystem `wj-querystring` percent-encodes form values. Prefer std wiring over
//! hand-rolled codecs once these resolve.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn encoding_url_encode_codegen_resolves() {
    let source = r#"
use std::encoding

pub fn encode_component(text: string) -> string {
    encoding.url_encode(text)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["encoding::url_encode"]);
}

#[test]
fn encoding_url_decode_codegen_resolves() {
    let source = r#"
use std::encoding

pub fn decode_component(text: string) -> Result<string, string> {
    encoding.url_decode(text)
}
"#;
    test_utils::assert_stdlib_runtime_links_any(
        source,
        &["encoding::url_decode", "encoding::url_decode_component"],
    );
}
