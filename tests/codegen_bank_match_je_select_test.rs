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

//! REGRESSION (LedgerKit R2.4): BankMatchRow JE <select> compose must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn bank_match_row_je_select_should_codegen() {
    let source = r##"
pub struct BankMatchRow {
    line_id: string,
    unmatched: bool,
    je_options_html: string,
}

impl BankMatchRow {
    pub fn new(line_id: string, unmatched: bool, je_options_html: string) -> BankMatchRow {
        BankMatchRow {
            line_id: line_id,
            unmatched: unmatched,
            je_options_html: je_options_html,
        }
    }
    pub fn render(self) -> string {
        if self.unmatched {
            "<span class=\"wj-bank-match-action\" data-wj-bank-match-cell>".to_string()
                + "<select data-wj-bank-match-je aria-label=\"Journal entry\">"
                + self.je_options_html
                + "</select>"
                + "<button type=\"button\" class=\"btn-secondary\" data-wj-bank-match data-line-id=\""
                + self.line_id
                + "\">Match</button></span>"
        } else {
            "<span class=\"muted\">Matched</span>".to_string()
        }
    }
}

fn main() {
    println!("{}", BankMatchRow::new(
        "bank~1000~demo-fit-01".to_string(),
        true,
        "<option value=\"seed-je-ops\">Ops</option>".to_string()
    ).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("data-wj-bank-match-je")
        && result.contains("data-wj-bank-match")
        && result.contains("seed-je-ops")
        && result.contains("wj-bank-match-action")
        && !result.contains("error[E");
    assert!(
        ok,
        "BankMatchRow JE select should codegen. Got:\n{}",
        result
    );
}
