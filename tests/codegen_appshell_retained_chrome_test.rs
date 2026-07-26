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

//! FAILING REPRO (LedgerKit ADR-001): AppShell retained chrome compose must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn appshell_retained_chrome_should_codegen() {
    let source = r#"
pub struct AppShell {
    brand: string,
    main_html: string,
}

impl AppShell {
    pub fn new() -> AppShell {
        AppShell { brand: "App".to_string(), main_html: "".to_string() }
    }
    pub fn brand(self, brand: string) -> AppShell {
        self.brand = brand
        self
    }
    pub fn main_html(self, html: string) -> AppShell {
        self.main_html = html
        self
    }
    pub fn render(self) -> string {
        format!("<div class='wj-app-shell'><strong>{}</strong><main id='main'>{}</main></div>", self.brand, self.main_html)
    }
}

fn main() {
    println!("{}", AppShell::new().brand("LedgerKit".to_string()).main_html("<p>x</p>".to_string()).render())
}
"#;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-app-shell")
        && (result.contains("AppShell") || result.contains("fn render"))
        && !result.contains("error[E");
    assert!(
        ok,
        "AppShell retained chrome compose should codegen. Got:\n{}",
        result
    );
}
