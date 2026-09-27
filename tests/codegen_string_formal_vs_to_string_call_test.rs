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
))]

//! Gate (dogfood):
//!
//! WJ declares `json_cors_error(status: int, message: string)` but codegen
//! demotes the formal to `message: &str` while downstream-style call sites
//! (`"…".to_string()` / `"…" + ""`) still pass `String` → rustc E0308.
//!
//! Formal and call site must agree on owned `String` (matching WJ `string`).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn owned_string_formal_must_not_demote_to_str_ref() {
    let source = r#"
pub struct ServerResponse {
    pub status: int,
    pub body: string,
}

fn error_json(message: string) -> string {
    "{\"error\":\"" + message + "\"}"
}

pub fn json_cors_error(status: int, message: string) -> ServerResponse {
    ServerResponse {
        status: status,
        body: error_json(message),
    }
}

pub fn missing_auth() -> ServerResponse {
    json_cors_error(401, "missing Authorization Bearer token" + "")
}

fn main() {
    let _ = missing_auth()
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("message: String")
            && !generated.contains("pub fn json_cors_error(status: i64, message: &str)"),
        "WJ `message: string` must stay owned String (not &str). Got:\n{generated}"
    );

    // Helper `error_json` may Phase-2 demote to `message: &str` + `error_json(&message)`.
    // That is not an E0308: the pub wrapper stays owned and the concat site passes String.
    let wrapper_demoted = generated.contains("pub fn json_cors_error(status: i64, message: &str)");
    let call_passes_string =
        generated.contains(".to_string()") || generated.contains("format!(");
    assert!(
        !wrapper_demoted || !call_passes_string,
        "&str json_cors_error formal + String call site is E0308. Got:\n{generated}"
    );
}
