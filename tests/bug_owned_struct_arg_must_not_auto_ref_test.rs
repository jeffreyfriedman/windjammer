//! Owned struct args must not be auto-ref'd into `&T` when the formal is owned `T`.
//!
//! Ecosystem `wj-form-parse` → `wj-multipart::parse(MultipartBody{…})` codegen emits
//! `parse(&MultipartBody{…})` plus field `&str` into `String` (E0308).

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
fn owned_struct_arg_must_not_emit_ampersand_at_call() {
    let source = r#"
pub struct MultipartBody {
    pub boundary: string,
    pub body: string,
}

fn parse(input: MultipartBody) -> int {
    strings_len(input.boundary) + strings_len(input.body)
}

fn strings_len(s: string) -> int {
    0
}

pub fn run(boundary: string, body: string) -> int {
    parse(MultipartBody {
        boundary: boundary,
        body: body,
    })
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "owned MultipartBody call must cargo-check, got:\n{generated}"
    );
    assert!(
        !generated.contains("parse(&MultipartBody")
            && !generated.contains("parse(& multipart")
            && !generated.contains("parse( &MultipartBody"),
        "must not auto-ref owned struct arg, got:\n{generated}"
    );
}
