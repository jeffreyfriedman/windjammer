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

//! FAILING REPRO (dogfood):
//!
//! `Err(failure) => Err(json_cors_error(failure.status, failure.message + ""))`
//! was emitting `json_cors_error(*failure.status, …)` → rustc E0614
//! (`i64` cannot be dereferenced). Copy field access through a match binding
//! is already a value in Rust — do not prefix `*`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn copy_int_field_from_match_err_must_not_be_star_deref() {
    let source = r#"
pub struct AuthFailure {
    pub status: int,
    pub message: string,
}

pub fn json_cors_error(status: int, message: string) -> string {
    message
}

pub fn resolve(result: Result<string, AuthFailure>) -> Result<string, string> {
    match result {
        Ok(slug) => Ok(slug),
        Err(failure) => Err(json_cors_error(failure.status, failure.message + "")),
    }
}

fn main() {
    let _ = resolve(Ok("demo" + ""))
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("json_cors_error(failure.status")
            || generated.contains("json_cors_error(failure.status,"),
        "expected bare failure.status at call site, got:\n{generated}"
    );
    assert!(
        !generated.contains("*failure.status"),
        "Copy int field must not be star-deref'd (E0614). Got:\n{generated}"
    );
}
