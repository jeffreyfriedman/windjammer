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

//! FAILING REPRO / REGRESSION (LedgerKit R2.5): unmatched CheckbookRow emits register→match link.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn checkbook_row_register_match_link_should_codegen() {
    let source = r##"
pub struct CheckbookRow {
    payee: string,
    line_id: string,
    unmatched: bool,
}

impl CheckbookRow {
    pub fn new(payee: string, line_id: string, unmatched: bool) -> CheckbookRow {
        CheckbookRow { payee: payee, line_id: line_id, unmatched: unmatched }
    }
    pub fn render(self) -> string {
        if self.unmatched {
            "<td>".to_string()
                + self.payee
                + " <button type=\"button\" class=\"btn-link\" data-wj-register-match data-line-id=\""
                + self.line_id
                + "\">Match</button></td>"
        } else {
            "<td>".to_string() + self.payee + "</td>"
        }
    }
}

fn main() {
    println!("{}", CheckbookRow::new(
        "OFFICE DEPOT".to_string(),
        "bank~1000~demo-fit-01".to_string(),
        true
    ).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("data-wj-register-match")
        && result.contains("data-line-id")
        && result.contains("demo-fit-01")
        && result.contains("Match")
        && !result.contains("error[E");
    assert!(
        ok,
        "CheckbookRow register→match link should codegen. Got:\n{}",
        result
    );
}
