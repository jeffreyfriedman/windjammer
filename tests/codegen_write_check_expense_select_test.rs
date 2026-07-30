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

//! FAILING REPRO (dogfood): WriteCheckForm expense account select must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn write_check_form_expense_select_should_codegen() {
    let source = r##"
pub struct WriteCheckForm {
    expense_code: string,
}

impl WriteCheckForm {
    pub fn new() -> WriteCheckForm {
        WriteCheckForm { expense_code: "5000".to_string() }
    }
    pub fn expense_code(self, code: string) -> WriteCheckForm {
        self.expense_code = code
        self
    }
    pub fn render(self) -> string {
        "<div class=\"wj-write-check-form\" data-wj-write-check data-wj-expense-code=\"".to_string()
            + self.expense_code
            + "\">"
            + "<select id=\"checkExpense\" data-wj-write-check-expense>"
            + "<option value=\"5000\">5000 · Payroll Expense</option>"
            + "<option value=\"5100\">5100 · Office Supplies</option>"
            + "</select>"
            + "</div>"
    }
}

fn main() {
    println!("{}", WriteCheckForm::new().expense_code("5100".to_string()).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("data-wj-write-check-expense")
        && result.contains("checkExpense")
        && result.contains("5100")
        && result.contains("wj-write-check-form")
        && !result.contains("error[E");
    assert!(
        ok,
        "WriteCheckForm expense select should codegen. Got:\n{}",
        result
    );
}
