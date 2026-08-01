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

//! FAILING REPRO (LedgerKit finance-screens dogfood after tip ownership work):
//!
//! `string` formals that are only read (passed to helpers / concatenated) must
//! demote to `&str`, never `&String`. Tip regenerating `home.wj` emitted
//! `render_kpi_grid(cash_html: &String, …)` which rejects `&'static str` call
//! sites in `dogfood_smoke.rs`.
//!
//! Contract: emitted signature uses `&str` (or owned `String`), not `&String`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn read_only_string_formals_must_not_emit_ref_string() {
    let source = r#"
pub fn render_kpi_grid(cash_html: string, ar_html: string, balanced: bool) -> string {
    let mark = if balanced {
        "ok".to_string()
    } else {
        "bad".to_string()
    }
    format!("<div>{}{}{}</div>", cash_html, ar_html, mark)
}

fn main() {
    println!("{}", render_kpi_grid("$1".to_string(), "$2".to_string(), true))
}
"#;

    let result = test_utils::compile_single(source);
    assert!(
        !result.contains("cash_html: &String")
            && !result.contains("ar_html: &String")
            && !result.contains(": &String,"),
        "read-only string formals must be &str (or owned String), not &String. Got:\n{result}"
    );
    assert!(
        result.contains("cash_html: &str")
            || result.contains("cash_html: String")
            || result.contains("cash_html: &str,"),
        "expected &str or owned String formal. Got:\n{result}"
    );
}
