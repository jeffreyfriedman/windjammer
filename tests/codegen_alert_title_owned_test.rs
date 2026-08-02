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

//! Gate (windjammer-ui Alert): `title` takes owned `string` in source,
//! but codegen historically emitted `title(&String)`, forcing awkward temps.
//! Desired: owned `String` (or `&str`) so `.title("Today".to_string())` works.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn alert_title_should_accept_owned_string() {
    let source = r#"
pub struct Alert {
    message: string,
    title: string,
}

impl Alert {
    pub fn warning(message: string) -> Alert {
        Alert { message: message, title: "".to_string() }
    }
    pub fn title(self, title: string) -> Alert {
        self.title = title
        self
    }
    pub fn render(self) -> string {
        format!("{}:{}", self.title, self.message)
    }
}

fn main() {
    let html = Alert::warning("Review".to_string()).title("Today".to_string()).render()
    println!("{}", html)
}
"#;

    let result = test_utils::compile_single(source);
    let owned_sig = result.contains("fn title")
        && result.contains("title: String")
        && !result.contains("title: &String");
    let chain_ok = !result.contains(".title(&")
        && (result.contains(".title(\"Today\".to_string())")
            || result.contains(".title(String::from")
            || result.contains("title: String"));
    assert!(
        owned_sig && chain_ok,
        "Alert::title should take owned String, not &String. Got:\n{}",
        result
    );
}
