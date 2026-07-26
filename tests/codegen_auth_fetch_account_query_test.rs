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

//! FAILING REPRO (LedgerKit R2): AuthFetch path must accept account query for register scope.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn auth_fetch_with_account_query_should_codegen() {
    let source = r##"
pub struct AuthFetch {
    path: string,
    kind: string,
}

impl AuthFetch {
    pub fn new(path: string, kind: string) -> AuthFetch {
        AuthFetch { path: path, kind: kind }
    }
    pub fn render(self) -> string {
        "<button data-wj-auth-fetch data-wj-fetch-path=\"".to_string()
            + self.path
            + "\" data-wj-render-kind=\""
            + self.kind
            + "\">Load</button>"
    }
}

fn main() {
    println!("{}", AuthFetch::new(
        "/api/v1/bank-imports/lines?account=1100".to_string(),
        "checkbook".to_string()
    ).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("data-wj-auth-fetch")
        && result.contains("account=1100")
        && result.contains("checkbook")
        && !result.contains("error[E");
    assert!(
        ok,
        "AuthFetch with account query should codegen. Got:\n{}",
        result
    );
}
