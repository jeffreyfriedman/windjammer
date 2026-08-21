//! `http::post(url, body)` runtime formals are `&str` / `&str`. Owned `String`
//! locals must borrow at both argument positions (ecosystem `wj-http-client`).

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
fn http_post_borrows_owned_url_and_body() {
    let source = r#"
use std::http

pub fn post_text(url: string, body: string) -> Result<string, string> {
    match http.post(url, body) {
        Ok(response) => response.text(),
        Err(e) => Err(e),
    }
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("http::post(&url, &body)"),
        "http::post must borrow owned String url and body, got:\n{generated}"
    );
}

#[test]
fn http_post_stdlib_sig_body_arg_is_borrowed_str() {
    let reg = windjammer::analyzer::SignatureRegistry::stdlib();
    let sig = reg
        .get_signature("http::post")
        .or_else(|| reg.get_fallback_signature("http::post"))
        .expect("http::post in stdlib registry");
    eprintln!(
        "post name={} nparams={} types={:?} own={:?} emitted={:?}",
        sig.name,
        sig.param_types.len(),
        sig.param_types,
        sig.param_ownership,
        sig.emitted_rust_ref_params
    );
    assert!(
        sig.param_types.len() >= 2,
        "runtime post is two-arg, got {:?}",
        sig.param_types
    );
    assert!(
        windjammer::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow(
            Some(sig),
            1,
        ),
        "body arg must need auto-borrow. types={:?} own={:?} emitted={:?}",
        sig.param_types,
        sig.param_ownership,
        sig.emitted_rust_ref_params
    );
}
