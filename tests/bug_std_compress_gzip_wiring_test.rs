//! `std::compress.gzip_encode` / `gzip_decode` must `cargo check` for HTTP middleware.
//!
//! Ecosystem `wj-compress` negotiates Accept-Encoding today; body codecs need this wiring.

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
fn std_compress_gzip_encode_codegen_resolves() {
    let source = r#"
use std::compress

pub fn encode(body: string) -> Result<string, string> {
    compress.gzip_encode(body)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["compress::gzip_encode"]);
}

#[test]
fn std_compress_gzip_decode_codegen_resolves() {
    let source = r#"
use std::compress

pub fn decode(body: string) -> Result<string, string> {
    compress.gzip_decode(body)
}
"#;
    test_utils::assert_stdlib_runtime_links_any(
        source,
        &["compress::gzip_decode", "compress::gunzip"],
    );
}
