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

//! Gate (dogfood): AuthFetch button compose must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn auth_fetch_button_should_codegen() {
    let source = r##"
pub struct AuthFetch {
    path: string,
    kind: string,
    label: string,
}

impl AuthFetch {
    pub fn new(path: string, kind: string) -> AuthFetch {
        AuthFetch {
            path: path,
            kind: kind,
            label: "Load".to_string(),
        }
    }
    pub fn label(self, label: string) -> AuthFetch {
        self.label = label
        self
    }
    pub fn render(self) -> string {
        "<button class=\"wj-auth-fetch\" data-wj-auth-fetch data-wj-fetch-path=\"".to_string()
            + self.path
            + "\" data-wj-render-kind=\""
            + self.kind
            + "\">"
            + self.label
            + "</button>"
    }
}

fn main() {
    println!("{}", AuthFetch::new("/api/v1/parties".to_string(), "parties".to_string()).label("Load parties".to_string()).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-auth-fetch")
        && result.contains("data-wj-auth-fetch")
        && !result.contains("error[E");
    assert!(
        ok,
        "AuthFetch compose should codegen. Got:\n{}",
        result
    );
}
