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

//! FAILING REPRO (LedgerKit R0): CheckbookRegister spent/received compose must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn checkbook_register_should_codegen() {
    let source = r##"
pub struct CheckbookRegister {
    account_label: string,
}

impl CheckbookRegister {
    pub fn new() -> CheckbookRegister {
        CheckbookRegister { account_label: "Cash".to_string() }
    }
    pub fn account_label(self, label: string) -> CheckbookRegister {
        self.account_label = label
        self
    }
    pub fn render(self) -> string {
        "<div class=\"wj-checkbook-register\"><th>Spent</th><th>Received</th>".to_string()
            + self.account_label
            + "</div>"
    }
}

fn main() {
    println!("{}", CheckbookRegister::new().account_label("1000".to_string()).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-checkbook-register")
        && result.contains("Spent")
        && !result.contains("error[E");
    assert!(
        ok,
        "CheckbookRegister compose should codegen. Got:\n{}",
        result
    );
}
