//! `std::encoding.form_parse` / `form_stringify` must wire for HTTP form bodies.
//!
//! Ecosystem `wj-querystring` is pure-WJ over `url_encode`/`url_decode`. Once these
//! resolve, thin-wrap the package (ordered pairs; repeated keys preserved).

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
fn encoding_form_parse_codegen_resolves() {
    let source = r#"
use std::encoding

pub fn load(text: string) -> Vec<(string, string)> {
    encoding.form_parse(text)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["encoding::form_parse"]);
}

#[test]
fn encoding_form_stringify_codegen_resolves() {
    let source = r#"
use std::encoding

pub fn dump(pairs: Vec<(string, string)>) -> string {
    encoding.form_stringify(pairs)
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["encoding::form_stringify"]);
}

#[test]
fn encoding_form_roundtrip_must_cargo_check() {
    let source = r#"
use std::encoding

pub fn roundtrip(text: string) -> string {
    let pairs = encoding.form_parse(text)
    encoding.form_stringify(pairs)
}
"#;
    test_utils::assert_stdlib_runtime_links(
        source,
        &["encoding::form_parse", "encoding::form_stringify"],
    );
}
