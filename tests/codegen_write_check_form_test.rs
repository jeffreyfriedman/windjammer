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

//! Gate (dogfood): WriteCheckForm compose must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn write_check_form_should_codegen() {
    let source = r##"
pub struct WriteCheckForm {
    sample_body: string,
}

impl WriteCheckForm {
    pub fn new() -> WriteCheckForm {
        WriteCheckForm { sample_body: "{}".to_string() }
    }
    pub fn sample_body(self, body: string) -> WriteCheckForm {
        self.sample_body = body
        self
    }
    pub fn render(self) -> string {
        "<div class=\"wj-write-check-form\" data-wj-write-check>".to_string()
            + "<input id=\"checkPayee\" type=\"text\"/>"
            + "<input id=\"checkAmount\" type=\"text\"/>"
            + "<textarea id=\"checkJeBody\">"
            + self.sample_body
            + "</textarea>"
            + "<button type=\"button\" id=\"postCheck\" data-wj-write-check-post>Post check</button>"
            + "</div>"
    }
}

fn main() {
    println!("{}", WriteCheckForm::new().sample_body("{\"lines\":[]}".to_string()).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-write-check-form")
        && result.contains("data-wj-write-check")
        && result.contains("checkPayee")
        && result.contains("checkJeBody")
        && result.contains("postCheck")
        && !result.contains("error[E");
    assert!(
        ok,
        "WriteCheckForm compose should codegen. Got:\n{}",
        result
    );
}
