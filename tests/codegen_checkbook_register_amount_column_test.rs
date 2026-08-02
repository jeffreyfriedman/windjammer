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

//! Gate (dogfood): CheckbookRegister Amount+Balance columns must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn checkbook_register_amount_column_should_codegen() {
    let source = r##"
pub struct CheckbookRow {
    payee: string,
    amount_html: string,
    balance_html: string,
}

impl CheckbookRow {
    pub fn new(payee: string, amount_html: string, balance_html: string) -> CheckbookRow {
        CheckbookRow { payee: payee, amount_html: amount_html, balance_html: balance_html }
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
            body = body + "<tr class=\"wj-checkbook-row\"><td>" + r.payee + "</td><td class=\"wj-num\">" + r.amount_html + "</td><td class=\"wj-num\">" + r.balance_html + "</td></tr>"
        }
        "<div class=\"wj-checkbook-register\"><th>Amount</th><th>Balance</th>".to_string() + body + "</div>"
    }
}

fn main() {
    println!("{}", CheckbookRegister::new().row(CheckbookRow::new("FEE".to_string(), "-45.00".to_string(), "100.00".to_string())).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-checkbook-register")
        && result.contains("Amount")
        && result.contains("Balance")
        && result.contains("wj-checkbook-row")
        && !result.contains(">Spent<")
        && !result.contains("error[E");
    assert!(
        ok,
        "CheckbookRegister Amount+Balance should codegen. Got:\n{}",
        result
    );
}
