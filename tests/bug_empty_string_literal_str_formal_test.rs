//! Empty string literal `""` into a demoted `&str` method formal must stay `""`
//! (or `&str`-compatible), not `String::new()` (E0308 in ecosystem tests).
//!
//! Ecosystem `wj-webhook` tests: `app.handle(..., "", "", "")` after formals
//! demote to `&str`.

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
fn empty_string_literal_into_str_ref_formal_is_not_string_new() {
    let source = r#"
pub struct App {
    secret: string,
}

impl App {
    pub fn new(secret: string) -> App {
        App { secret: secret }
    }

    pub fn handle(self, token: string, body: string) -> string {
        if token == self.secret {
            return body
        }
        ""
    }
}

pub fn call(app: App) -> string {
    app.handle("", "")
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("handle(String::new()")
            && !generated.contains("handle(String::new(), String::new())"),
        "empty string literal must not become String::new() for &str formals, got:\n{generated}"
    );
    // Accept "" or "".to_string() into owned String formals
    let ok = generated.contains("handle(\"\"")
        || generated.contains("handle(\"\".to_string()")
        || (generated.contains("token: String") && generated.contains("handle("));
    assert!(
        ok || generated.contains("fn handle") && !generated.contains("String::new()"),
        "expected empty-literal-friendly call site, got:\n{generated}"
    );
}
