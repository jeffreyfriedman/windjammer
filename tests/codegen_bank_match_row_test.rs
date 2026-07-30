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

//! REGRESSION (dogfood): BankMatchRow Match button must codegen.
//! FAILING REPRO: `&'static str` raw-string JS helper (hand-patched in windjammer-ui today).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn bank_match_row_should_codegen() {
    let source = r##"
pub struct BankMatchRow {
    line_id: string,
    unmatched: bool,
}

impl BankMatchRow {
    pub fn new(line_id: string, unmatched: bool) -> BankMatchRow {
        BankMatchRow { line_id: line_id, unmatched: unmatched }
    }
    pub fn render(self) -> string {
        if self.unmatched {
            "<button type=\"button\" class=\"btn-secondary\" data-wj-bank-match data-line-id=\"".to_string()
                + self.line_id
                + "\">Match</button>"
        } else {
            "<span class=\"muted\">Matched</span>".to_string()
        }
    }
}

fn main() {
    println!("{}", BankMatchRow::new("bank~1000~demo-fit-01".to_string(), true).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("data-wj-bank-match")
        && result.contains("data-line-id")
        && result.contains("Match")
        && result.contains("demo-fit-01")
        && !result.contains("error[E");
    assert!(
        ok,
        "BankMatchRow Match button should codegen. Got:\n{}",
        result
    );
}

#[test]
fn bank_match_static_str_runtime_helper_should_codegen() {
    // Hand patch uses: pub fn bank_match_runtime_js() -> &'static str { r##"..."## }
    let source = r####"
pub fn bank_match_runtime_js() -> &'static str {
    r##"
(function () {
  if (window.__wjBankMatchBound) return;
  window.__wjBankMatchBound = true;
})();
"##
}

fn main() {
    let _ = bank_match_runtime_js();
}
"####;

    let result = test_utils::compile_single(source);
    let ok = result.contains("bank_match_runtime_js")
        && result.contains("__wjBankMatchBound")
        && (result.contains("&'static str") || result.contains("&'static str"))
        && result.contains("pub fn bank_match_runtime_js")
        && !result.contains("error[E")
        && !result.contains("error:");
    assert!(
        ok,
        "FAILING: &'static str raw-string bank_match_runtime_js should codegen (hand-patched today). Got:\n{}",
        result
    );
}
