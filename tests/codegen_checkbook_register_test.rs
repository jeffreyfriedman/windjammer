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

//! FAILING REPRO (LedgerKit R0+): CheckbookRegister row compose must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn checkbook_register_should_codegen() {
    let source = r##"
pub struct CheckbookRow {
    payee: string,
    spent_html: string,
}

impl CheckbookRow {
    pub fn new(payee: string, spent_html: string) -> CheckbookRow {
        CheckbookRow { payee: payee, spent_html: spent_html }
    }
}

pub struct CheckbookRegister {
    rows: Vec<CheckbookRow>,
}

impl CheckbookRegister {
    pub fn new() -> CheckbookRegister {
        CheckbookRegister { rows: Vec::new() }
    }
    pub fn row(self, row: CheckbookRow) -> CheckbookRegister {
        self.rows.push(row)
        self
    }
    pub fn render(self) -> string {
        let mut body = "".to_string()
        for r in self.rows {
            body = body + "<tr class=\"wj-checkbook-row\">" + r.payee + r.spent_html + "</tr>"
        }
        "<div class=\"wj-checkbook-register\"><th>Spent</th><th>Received</th>".to_string() + body + "</div>"
    }
}

fn main() {
    println!("{}", CheckbookRegister::new().row(CheckbookRow::new("FEE".to_string(), "$45.00".to_string())).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-checkbook-register")
        && result.contains("Spent")
        && result.contains("wj-checkbook-row")
        && !result.contains("error[E");
    assert!(
        ok,
        "CheckbookRegister row compose should codegen. Got:\n{}",
        result
    );
}
