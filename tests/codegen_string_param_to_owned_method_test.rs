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

//! FAILING REPRO (LedgerKit dogfood): `string` fn params must codegen as owned
//! `String` (or auto `.to_string()` at call sites) when passed into methods that
//! take `String` — not bare `&str`.
//!
//! Seen in finance-screens Home:
//!   pub fn render_kpi_grid(cash_html: string, ...) {
//!     KpiTile::new(...).value_html(cash_html)  // value_html expects String
//!   }
//! Codegen emitted `cash_html: &str` + `.value_html(cash_html)` → E0308.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn string_param_passed_to_owned_method_should_compile() {
    let source = r#"
pub struct KpiTile { value_html: string }

impl KpiTile {
    pub fn new() -> KpiTile {
        KpiTile { value_html: "".to_string() }
    }
    pub fn value_html(self, html: string) -> KpiTile {
        self.value_html = html
        self
    }
    pub fn render(self) -> string {
        self.value_html
    }
}

pub fn render_kpi(cash_html: string) -> string {
    KpiTile::new().value_html(cash_html).render()
}

fn main() {
    println!("{}", render_kpi("$1".to_string()))
}
"#;

    let result = test_utils::compile_single(source);
    let has_type_err = result.contains("error[E0308]")
        || result.contains("expected `String`, found `&str`")
        || result.contains("expected String, found &str");
    let looks_ok = result.contains("fn render_kpi")
        && !has_type_err
        && (result.contains("cash_html: String")
            || result.contains("value_html(cash_html.to_string())")
            || result.contains("value_html(cash_html.clone())")
            || (result.contains("value_html(cash_html)") && result.contains("cash_html: String")));

    assert!(
        looks_ok,
        "string param → owned String method arg must typecheck. Got:\n{}",
        result
    );
}
