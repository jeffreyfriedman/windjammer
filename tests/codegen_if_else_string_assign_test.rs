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

//! FAILING REPRO (web-run warning): mut string initialized then always reassigned.
//!
//! Windjammer-ui MoneyDisplay `format_cents` currently codegen's to:
//!   let mut rem_s = "".to_string();
//!   if rem < 10 { rem_s = format!("0{}", rem); } else { rem_s = format!("{}", rem); }
//! which triggers rustc `unused_assignments` on the initial "".
//!
//! Desired: emit an if-expression binding:
//!   let rem_s = if rem < 10 { format!("0{}", rem) } else { format!("{}", rem) };

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn if_else_string_assign_should_codegen_as_expression() {
    let source = r#"
pub fn pad_cents(rem: int) -> string {
    let mut rem_s = "".to_string()
    if rem < 10 {
        rem_s = format!("0{}", rem)
    } else {
        rem_s = format!("{}", rem)
    }
    rem_s
}

fn main() {
    let _ = pad_cents(3)
}
"#;

    let result = test_utils::compile_single(source);

    // Prefer expression form — no dead initial assignment.
    let has_dead_init = result.contains("let mut rem_s = \"\".to_string()")
        || result.contains("let mut rem_s = String::new()")
        || result.contains("let mut rem_s = \"\".to_string();");

    let has_expr_form = result.contains("let rem_s = if")
        || result.contains("let rem_s: String = if");

    assert!(
        has_expr_form || !has_dead_init,
        "codegen should avoid dead mut string init before if/else assign. Got:\n{}",
        result
    );
}
